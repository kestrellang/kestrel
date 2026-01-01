//! Type parameter parsing for generics
//!
//! This module handles parsing of:
//! - Type parameter lists: `[T, U, V]`
//! - Type parameters with defaults: `[T, U = Int]`
//! - Where clauses: `where T: Proto and Proto2, U: Other`
//! - Type argument lists: `[Int, String]` in type use positions
//! - Conformance lists: `: Proto1, Proto2` for structs and protocols

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::SyntaxKind;

use crate::common::skip_trivia;
use crate::event::EventSink;
use crate::input::{to_kestrel_span, ParserExtra, ParserInput};

/// Raw parsed data for a single type parameter
/// Syntax: T or T = Default
#[derive(Debug, Clone)]
pub struct TypeParameterData {
    /// The name of the type parameter
    pub name: Span,
    /// Optional default type (path segments)
    pub default: Option<Vec<Span>>,
}

/// Raw parsed data for a type argument (in use position)
/// Syntax: SomeType or SomeType[Args]
#[derive(Debug, Clone)]
pub struct TypeArgumentData {
    /// Path segments for the type
    pub path: Vec<Span>,
    /// Optional type arguments for this type
    pub args: Option<Vec<TypeArgumentData>>,
}

/// Raw parsed data for a type bound
/// Syntax: T: Proto and Proto2 or T: Container[Int] or T.Item: Proto
#[derive(Debug, Clone)]
pub struct TypeBoundData {
    /// The constrained path - either simple (T) or associated type path (T.Item)
    pub path: Vec<Span>,
    /// The protocols/bounds (connected by `and`), with optional type arguments
    pub bounds: Vec<TypeArgumentData>,
}

/// Raw parsed data for a negative type bound
/// Syntax: T: not Copyable
#[derive(Debug, Clone)]
pub struct NegativeTypeBoundData {
    /// The constrained path - typically just the type parameter name (T)
    pub path: Vec<Span>,
    /// The `not` keyword span
    pub not_span: Span,
    /// The negated bound (the protocol this type does NOT need to satisfy)
    pub bound: TypeArgumentData,
}

/// Raw parsed data for a type equality constraint
/// Syntax: T.Item = Type or T.Item = U.Item
#[derive(Debug, Clone)]
pub struct TypeEqualityData {
    /// The left side path (e.g., T.Item)
    pub left: Vec<Span>,
    /// The equals span (=)
    pub equals_span: Span,
    /// The right side type
    pub right: crate::ty::TyVariant,
}

/// A constraint in a where clause - either a bound, negative bound, or equality
#[derive(Debug, Clone)]
pub enum WhereConstraintData {
    /// Type bound: T: Proto or T.Item: Proto
    Bound(TypeBoundData),
    /// Negative type bound: T: not Copyable
    NegativeBound(NegativeTypeBoundData),
    /// Type equality: T.Item = Type
    Equality(TypeEqualityData),
}

/// Raw parsed data for a where clause
/// Syntax: where T: Proto, U: Other, T.Item = Int
#[derive(Debug, Clone)]
pub struct WhereClauseData {
    /// The `where` keyword span
    pub where_span: Span,
    /// The constraints (bounds and equalities)
    pub constraints: Vec<WhereConstraintData>,
}

/// Parser for a path (used in type positions): Ident or Ident.Ident.Ident
fn path_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, Vec<Span>, ParserExtra<'tokens>> + Clone {
    skip_trivia().ignore_then(
        select! {
            Token::Identifier = e => to_kestrel_span(e.span()),
        }
        .separated_by(just(Token::Dot))
        .at_least(1)
        .collect(),
    )
}

/// Parser for a single type argument (recursive to handle nested generics)
fn type_argument_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, TypeArgumentData, ParserExtra<'tokens>> + Clone {
    recursive(|type_arg| {
        // A single type argument: Path optionally followed by [Args]
        path_parser()
            .then(
                skip_trivia()
                    .ignore_then(just(Token::LBracket))
                    .ignore_then(
                        type_arg
                            .clone()
                            .separated_by(just(Token::Comma))
                            .allow_trailing()
                            .collect::<Vec<_>>(),
                    )
                    .then_ignore(skip_trivia())
                    .then_ignore(just(Token::RBracket))
                    .or_not(),
            )
            .map(|(path, args)| TypeArgumentData { path, args })
    })
}

/// Parser for type arguments: [Type, Type, ...]
/// Handles nested type arguments like Foo[Bar[Baz]]
pub fn type_argument_list_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, Vec<TypeArgumentData>, ParserExtra<'tokens>> + Clone
{
    skip_trivia()
        .ignore_then(just(Token::LBracket))
        .ignore_then(
            type_argument_parser()
                .separated_by(just(Token::Comma))
                .allow_trailing()
                .collect(),
        )
        .then_ignore(skip_trivia())
        .then_ignore(just(Token::RBracket))
}

/// Parser for type arguments with bracket spans: [Type, Type, ...]
/// Returns (lbracket, args, rbracket)
pub fn type_argument_list_with_spans_parser<'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens>,
    (Span, Vec<TypeArgumentData>, Span),
    ParserExtra<'tokens>,
> + Clone {
    skip_trivia()
        .ignore_then(just(Token::LBracket).map_with(|_, e| to_kestrel_span(e.span())))
        .then(
            type_argument_parser()
                .separated_by(just(Token::Comma))
                .allow_trailing()
                .collect(),
        )
        .then_ignore(skip_trivia())
        .then(just(Token::RBracket).map_with(|_, e| to_kestrel_span(e.span())))
        .map(|((lbracket, args), rbracket)| (lbracket, args, rbracket))
}

/// Parser for optional type arguments after a path
/// Returns (path, optional args)
pub fn path_with_optional_args_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, TypeArgumentData, ParserExtra<'tokens>> + Clone {
    path_parser()
        .then(type_argument_list_parser().or_not())
        .map(|(path, args)| TypeArgumentData { path, args })
}

/// Parser for a single type parameter: T or T = Default
fn type_parameter_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, TypeParameterData, ParserExtra<'tokens>> + Clone {
    skip_trivia()
        .ignore_then(select! {
            Token::Identifier = e => to_kestrel_span(e.span()),
        })
        .then(
            // Optional default: = Type
            skip_trivia()
                .ignore_then(just(Token::Equals))
                .ignore_then(path_parser())
                .or_not(),
        )
        .map(|(name, default)| TypeParameterData { name, default })
}

/// Parser for type parameter list: [T, U, V] or [T, U = String]
pub fn type_parameter_list_parser<'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens>,
    (Span, Vec<TypeParameterData>, Span),
    ParserExtra<'tokens>,
> + Clone {
    skip_trivia()
        .ignore_then(just(Token::LBracket).map_with(|_, e| to_kestrel_span(e.span())))
        .then(
            type_parameter_parser()
                .separated_by(just(Token::Comma))
                .allow_trailing()
                .collect(),
        )
        .then(
            skip_trivia()
                .ignore_then(just(Token::RBracket).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .map(|((lbracket, params), rbracket)| (lbracket, params, rbracket))
}

/// Parser for a single positive type bound: T: Proto and Proto2 or T.Item: Proto
fn positive_type_bound_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, TypeBoundData, ParserExtra<'tokens>> + Clone {
    skip_trivia()
        .ignore_then(path_parser())
        .then_ignore(skip_trivia())
        .then_ignore(just(Token::Colon))
        .then_ignore(skip_trivia())
        // Ensure we don't start with `not` - that's a negative bound
        .then(
            // Bounds separated by `and`, with optional type arguments
            path_with_optional_args_parser()
                .separated_by(
                    skip_trivia()
                        .ignore_then(just(Token::And))
                        .ignore_then(skip_trivia()),
                )
                .at_least(1)
                .collect(),
        )
        .map(|(path, bounds)| TypeBoundData { path, bounds })
}

/// Parser for a negative type bound: T: not Copyable
fn negative_type_bound_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, NegativeTypeBoundData, ParserExtra<'tokens>> + Clone
{
    skip_trivia()
        .ignore_then(path_parser())
        .then_ignore(skip_trivia())
        .then_ignore(just(Token::Colon))
        .then_ignore(skip_trivia())
        .then(just(Token::Not).map_with(|_, e| to_kestrel_span(e.span())))
        .then_ignore(skip_trivia())
        .then(path_with_optional_args_parser())
        .map(|((path, not_span), bound)| NegativeTypeBoundData {
            path,
            not_span,
            bound,
        })
}

/// Parser for a type equality constraint: T.Item = Type
fn type_equality_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, TypeEqualityData, ParserExtra<'tokens>> + Clone {
    skip_trivia()
        .ignore_then(path_parser())
        .then_ignore(skip_trivia())
        .then(just(Token::Equals).map_with(|_, e| to_kestrel_span(e.span())))
        .then_ignore(skip_trivia())
        .then(crate::ty::ty_parser())
        .map(|((left, equals_span), right)| TypeEqualityData {
            left,
            equals_span,
            right,
        })
}

/// Parser for a single where clause constraint (bound, negative bound, or equality)
fn where_constraint_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, WhereConstraintData, ParserExtra<'tokens>> + Clone {
    // Try equality first (T.Item = Type), then negative bound (T: not Proto),
    // then positive bound (T: Proto)
    // This order matters because path_parser is greedy and negative bounds
    // must be tried before positive bounds
    type_equality_parser()
        .map(WhereConstraintData::Equality)
        .or(negative_type_bound_parser().map(WhereConstraintData::NegativeBound))
        .or(positive_type_bound_parser().map(WhereConstraintData::Bound))
}

/// Parser for where clause: where T: Proto, U: Other, T.Item = Int
pub fn where_clause_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, WhereClauseData, ParserExtra<'tokens>> + Clone {
    skip_trivia()
        .ignore_then(just(Token::Where).map_with(|_, e| to_kestrel_span(e.span())))
        .then(
            where_constraint_parser()
                .separated_by(just(Token::Comma))
                .at_least(1)
                .collect(),
        )
        .map(|(where_span, constraints)| WhereClauseData {
            where_span,
            constraints,
        })
}

/// Parser for a single conformance item: Proto or not Proto
fn conformance_item_parser<'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens>,
    crate::common::ConformanceItemData,
    ParserExtra<'tokens>,
> + Clone {
    skip_trivia()
        .ignore_then(
            just(Token::Not)
                .map_with(|_, e| to_kestrel_span(e.span()))
                .or_not(),
        )
        .then(crate::ty::ty_parser())
        .map(|(not_span, ty)| crate::common::ConformanceItemData { not_span, ty })
}

/// Parser for conformance list: : Proto1, Proto2[T], not Copyable
/// Used after struct/protocol names to declare conformance/inheritance
pub fn conformance_list_parser<'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens>,
    (Span, Vec<crate::common::ConformanceItemData>),
    ParserExtra<'tokens>,
> + Clone {
    skip_trivia()
        .ignore_then(just(Token::Colon).map_with(|_, e| to_kestrel_span(e.span())))
        .then(
            conformance_item_parser()
                .separated_by(just(Token::Comma))
                .at_least(1)
                .collect(),
        )
        .map(|(colon_span, conformances)| (colon_span, conformances))
}

/// Emit events for a type parameter list
pub fn emit_type_parameter_list(
    sink: &mut EventSink,
    lbracket: Span,
    params: Vec<TypeParameterData>,
    rbracket: Span,
) {
    sink.start_node(SyntaxKind::TypeParameterList);
    sink.add_token(SyntaxKind::LBracket, lbracket);

    for param in params {
        emit_type_parameter(sink, param);
    }

    sink.add_token(SyntaxKind::RBracket, rbracket);
    sink.finish_node();
}

/// Emit events for a single type parameter
fn emit_type_parameter(sink: &mut EventSink, param: TypeParameterData) {
    sink.start_node(SyntaxKind::TypeParameter);

    // Emit name
    sink.start_node(SyntaxKind::Name);
    sink.add_token(SyntaxKind::Identifier, param.name);
    sink.finish_node();

    // Emit default if present
    if let Some(default_path) = param.default {
        sink.start_node(SyntaxKind::DefaultType);
        // Wrap in Ty -> TyPath -> Path for consistency with type extractor
        sink.start_node(SyntaxKind::Ty);
        sink.start_node(SyntaxKind::TyPath);
        emit_path(sink, &default_path);
        sink.finish_node(); // TyPath
        sink.finish_node(); // Ty
        sink.finish_node(); // DefaultType
    }

    sink.finish_node();
}

/// Emit events for a where clause
pub fn emit_where_clause(sink: &mut EventSink, data: WhereClauseData) {
    sink.start_node(SyntaxKind::WhereClause);
    sink.add_token(SyntaxKind::Where, data.where_span);

    for constraint in data.constraints {
        match constraint {
            WhereConstraintData::Bound(bound) => emit_type_bound(sink, bound),
            WhereConstraintData::NegativeBound(neg_bound) => {
                emit_negative_type_bound(sink, neg_bound)
            }
            WhereConstraintData::Equality(equality) => emit_type_equality(sink, equality),
        }
    }

    sink.finish_node();
}

/// Emit events for a type equality constraint
fn emit_type_equality(sink: &mut EventSink, equality: TypeEqualityData) {
    sink.start_node(SyntaxKind::TypeEquality);

    // Emit the left path (T.Item)
    sink.start_node(SyntaxKind::AssociatedTypeTarget);
    emit_path(sink, &equality.left);
    sink.finish_node();

    // Emit =
    sink.add_token(SyntaxKind::Equals, equality.equals_span);

    // Emit the right type
    crate::ty::emit_ty_variant(sink, &equality.right);

    sink.finish_node();
}

/// Emit events for a conformance list: : Proto1, Proto2, not Copyable
pub fn emit_conformance_list(
    sink: &mut EventSink,
    colon_span: Span,
    conformances: &[crate::common::ConformanceItemData],
) {
    sink.start_node(SyntaxKind::ConformanceList);
    sink.add_token(SyntaxKind::Colon, colon_span);

    for conformance in conformances {
        sink.start_node(SyntaxKind::ConformanceItem);

        // If this is a negative conformance, wrap in NegativeConformance node
        if let Some(not_span) = &conformance.not_span {
            sink.start_node(SyntaxKind::NegativeConformance);
            sink.add_token(SyntaxKind::Not, not_span.clone());
            crate::ty::emit_ty_variant(sink, &conformance.ty);
            sink.finish_node();
        } else {
            crate::ty::emit_ty_variant(sink, &conformance.ty);
        }

        sink.finish_node();
    }

    sink.finish_node();
}

/// Emit events for a type bound
fn emit_type_bound(sink: &mut EventSink, bound: TypeBoundData) {
    sink.start_node(SyntaxKind::TypeBound);

    // Emit the constrained path (T or T.Item)
    if bound.path.len() == 1 {
        // Simple type parameter: T
        sink.start_node(SyntaxKind::Name);
        sink.add_token(SyntaxKind::Identifier, bound.path[0].clone());
        sink.finish_node();
    } else {
        // Associated type path: T.Item or similar
        sink.start_node(SyntaxKind::AssociatedTypeTarget);
        emit_path(sink, &bound.path);
        sink.finish_node();
    }

    // Emit each bound with optional type arguments
    for bound_type in bound.bounds {
        emit_path(sink, &bound_type.path);

        // Emit type arguments if present (e.g., Container[T])
        if let Some(ref type_args) = bound_type.args {
            emit_type_argument_list(sink, type_args);
        }
    }

    sink.finish_node();
}

/// Emit events for a negative type bound: T: not Copyable
fn emit_negative_type_bound(sink: &mut EventSink, bound: NegativeTypeBoundData) {
    sink.start_node(SyntaxKind::TypeBound);

    // Emit the constrained path (T)
    if bound.path.len() == 1 {
        // Simple type parameter: T
        sink.start_node(SyntaxKind::Name);
        sink.add_token(SyntaxKind::Identifier, bound.path[0].clone());
        sink.finish_node();
    } else {
        // Associated type path: T.Item or similar
        sink.start_node(SyntaxKind::AssociatedTypeTarget);
        emit_path(sink, &bound.path);
        sink.finish_node();
    }

    // Wrap the negated bound in NegativeConformance
    sink.start_node(SyntaxKind::NegativeConformance);
    sink.add_token(SyntaxKind::Not, bound.not_span);

    // Emit the bound (e.g., Copyable) as a path
    emit_path(sink, &bound.bound.path);

    // Emit type arguments if present
    if let Some(ref type_args) = bound.bound.args {
        emit_type_argument_list(sink, type_args);
    }

    sink.finish_node(); // NegativeConformance

    sink.finish_node(); // TypeBound
}

/// Emit events for type argument list
pub fn emit_type_argument_list(sink: &mut EventSink, args: &[TypeArgumentData]) {
    sink.start_node(SyntaxKind::TypeArgumentList);

    for arg in args {
        emit_type_argument(sink, arg);
    }

    sink.finish_node();
}

/// Emit events for a single type argument
fn emit_type_argument(sink: &mut EventSink, arg: &TypeArgumentData) {
    sink.start_node(SyntaxKind::Ty);
    sink.start_node(SyntaxKind::TyPath);
    emit_path(sink, &arg.path);

    // Emit nested type arguments if present
    if let Some(ref nested_args) = arg.args {
        emit_type_argument_list(sink, nested_args);
    }

    sink.finish_node(); // TyPath
    sink.finish_node(); // Ty
}

/// Helper to emit a path
fn emit_path(sink: &mut EventSink, segments: &[Span]) {
    sink.start_node(SyntaxKind::Path);

    let mut prev_end: Option<usize> = None;
    for span in segments.iter() {
        if let Some(end) = prev_end {
            // Add dot separator - span is between previous segment's end and current segment's start
            // The dot should be a single character somewhere in this gap
            let dot_start = end;
            let dot_end = (end + 1).min(span.start);
            sink.add_token(SyntaxKind::Dot, Span::from(dot_start..dot_end));
        }
        sink.start_node(SyntaxKind::PathElement);
        sink.add_token(SyntaxKind::Identifier, span.clone());
        sink.finish_node();
        prev_end = Some(span.end);
    }

    sink.finish_node();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{create_input, prepare_tokens};
    use kestrel_lexer::lex;

    fn parse_type_params(source: &str) -> Option<(Span, Vec<TypeParameterData>, Span)> {
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();
        let prepared = prepare_tokens(tokens.into_iter());
        let input = create_input(&prepared, source.len());
        type_parameter_list_parser().parse(input).into_result().ok()
    }

    fn parse_where(source: &str) -> Option<WhereClauseData> {
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();
        let prepared = prepare_tokens(tokens.into_iter());
        let input = create_input(&prepared, source.len());
        where_clause_parser().parse(input).into_result().ok()
    }

    fn parse_type_args(source: &str) -> Option<Vec<TypeArgumentData>> {
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();
        let prepared = prepare_tokens(tokens.into_iter());
        let input = create_input(&prepared, source.len());
        type_argument_list_parser().parse(input).into_result().ok()
    }

    #[test]
    fn test_single_type_parameter() {
        let result = parse_type_params("[T]");
        assert!(result.is_some());
        let (_, params, _) = result.unwrap();
        assert_eq!(params.len(), 1);
        assert!(params[0].default.is_none());
    }

    #[test]
    fn test_multiple_type_parameters() {
        let result = parse_type_params("[T, U, V]");
        assert!(result.is_some());
        let (_, params, _) = result.unwrap();
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn test_type_parameter_with_default() {
        let result = parse_type_params("[T = Int]");
        assert!(result.is_some());
        let (_, params, _) = result.unwrap();
        assert_eq!(params.len(), 1);
        assert!(params[0].default.is_some());
    }

    #[test]
    fn test_mixed_type_parameters() {
        let result = parse_type_params("[K, V = String]");
        assert!(result.is_some());
        let (_, params, _) = result.unwrap();
        assert_eq!(params.len(), 2);
        assert!(params[0].default.is_none());
        assert!(params[1].default.is_some());
    }

    #[test]
    fn test_where_clause_single_bound() {
        let result = parse_where("where T: Equatable");
        assert!(result.is_some());
        let data = result.unwrap();
        assert_eq!(data.constraints.len(), 1);
        match &data.constraints[0] {
            WhereConstraintData::Bound(bound) => assert_eq!(bound.bounds.len(), 1),
            _ => panic!("Expected Bound constraint"),
        }
    }

    #[test]
    fn test_where_clause_and_bounds() {
        let result = parse_where("where T: Equatable and Hashable");
        assert!(result.is_some());
        let data = result.unwrap();
        assert_eq!(data.constraints.len(), 1);
        match &data.constraints[0] {
            WhereConstraintData::Bound(bound) => assert_eq!(bound.bounds.len(), 2),
            _ => panic!("Expected Bound constraint"),
        }
    }

    #[test]
    fn test_where_clause_multiple_params() {
        let result = parse_where("where T: Equatable, U: Serializable");
        assert!(result.is_some());
        let data = result.unwrap();
        assert_eq!(data.constraints.len(), 2);
    }

    #[test]
    fn test_where_clause_negative_bound() {
        let result = parse_where("where T: not Copyable");
        assert!(result.is_some(), "Failed to parse 'where T: not Copyable'");
        let data = result.unwrap();
        assert_eq!(data.constraints.len(), 1);
        match &data.constraints[0] {
            WhereConstraintData::NegativeBound(neg_bound) => {
                assert_eq!(neg_bound.path.len(), 1); // T
            }
            _ => panic!("Expected NegativeBound constraint"),
        }
    }

    #[test]
    fn test_where_clause_mixed_positive_negative() {
        let result = parse_where("where T: Equatable, U: not Copyable");
        assert!(result.is_some(), "Failed to parse mixed bounds");
        let data = result.unwrap();
        assert_eq!(data.constraints.len(), 2);
        match &data.constraints[0] {
            WhereConstraintData::Bound(_) => {}
            _ => panic!("Expected Bound for first constraint"),
        }
        match &data.constraints[1] {
            WhereConstraintData::NegativeBound(_) => {}
            _ => panic!("Expected NegativeBound for second constraint"),
        }
    }

    #[test]
    fn test_type_arguments_simple() {
        let result = parse_type_args("[Int]");
        assert!(result.is_some());
        let args = result.unwrap();
        assert_eq!(args.len(), 1);
        assert!(args[0].args.is_none());
    }

    #[test]
    fn test_type_arguments_multiple() {
        let result = parse_type_args("[Int, String]");
        assert!(result.is_some());
        let args = result.unwrap();
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn test_type_arguments_nested() {
        let result = parse_type_args("[List[Int]]");
        assert!(result.is_some());
        let args = result.unwrap();
        assert_eq!(args.len(), 1);
        assert!(args[0].args.is_some());
        let nested = args[0].args.as_ref().unwrap();
        assert_eq!(nested.len(), 1);
    }

    #[test]
    fn test_type_arguments_deeply_nested() {
        let result = parse_type_args("[Map[String, List[Int]]]");
        assert!(result.is_some());
        let args = result.unwrap();
        assert_eq!(args.len(), 1);
        let nested = args[0].args.as_ref().unwrap();
        assert_eq!(nested.len(), 2); // String and List[Int]
    }

    fn parse_conformances(source: &str) -> Option<(Span, Vec<crate::common::ConformanceItemData>)> {
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();
        let prepared = prepare_tokens(tokens.into_iter());
        let input = create_input(&prepared, source.len());
        conformance_list_parser().parse(input).into_result().ok()
    }

    #[test]
    fn test_conformance_single() {
        let result = parse_conformances(": Drawable");
        assert!(result.is_some());
        let (_, conformances) = result.unwrap();
        assert_eq!(conformances.len(), 1);
    }

    #[test]
    fn test_conformance_multiple() {
        let result = parse_conformances(": Drawable, Equatable");
        assert!(result.is_some());
        let (_, conformances) = result.unwrap();
        assert_eq!(conformances.len(), 2);
    }

    #[test]
    fn test_conformance_generic() {
        let result = parse_conformances(": Iterator[Int]");
        assert!(result.is_some());
        let (_, conformances) = result.unwrap();
        assert_eq!(conformances.len(), 1);
    }

    #[test]
    fn test_conformance_path() {
        let result = parse_conformances(": Std.Core.Equatable");
        assert!(result.is_some());
        let (_, conformances) = result.unwrap();
        assert_eq!(conformances.len(), 1);
    }

    #[test]
    fn test_conformance_negative() {
        let result = parse_conformances(": not Copyable");
        assert!(result.is_some(), "Failed to parse ': not Copyable'");
        let (_, conformances) = result.unwrap();
        assert_eq!(conformances.len(), 1);
        assert!(
            conformances[0].not_span.is_some(),
            "Expected not_span to be Some"
        );
    }

    #[test]
    fn test_conformance_mixed_positive_negative() {
        let result = parse_conformances(": Resource, not Copyable");
        assert!(
            result.is_some(),
            "Failed to parse ': Resource, not Copyable'"
        );
        let (_, conformances) = result.unwrap();
        assert_eq!(conformances.len(), 2);
        assert!(
            conformances[0].not_span.is_none(),
            "Resource should have not_span None"
        );
        assert!(
            conformances[1].not_span.is_some(),
            "not Copyable should have not_span Some"
        );
    }
}
