use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::common::parsers::contextual_keyword;
use crate::event::{EventSink, TreeBuilder};
use crate::input::{ParserExtra, ParserInput, to_kestrel_span};

/// The keyword prefix that names a function type's *kind*
/// (`mutating (T) -> R`, `consuming (T) -> R`, `escaping (T) -> R`).
///
/// Parser-local mirror of `kestrel_ast::FnTypeKind` minus its `Normal`
/// variant — the parser only records a prefix that is *present*, and
/// kestrel-parser deliberately does not depend on kestrel-ast (the CST is the
/// only contract between them). The AST builder maps the emitted token back
/// to `FnTypeKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FnKindPrefix {
    Mutating,
    Consuming,
    Escaping,
}

impl FnKindPrefix {
    /// The CST token kind this prefix is emitted as. `escaping` is a
    /// *contextual* keyword — it lexes as an ordinary `Identifier` and stays
    /// a legal identifier everywhere else — so it round-trips as `Identifier`
    /// and the AST builder matches it by source text.
    fn syntax_kind(self) -> SyntaxKind {
        match self {
            FnKindPrefix::Mutating => SyntaxKind::Mutating,
            FnKindPrefix::Consuming => SyntaxKind::Consuming,
            FnKindPrefix::Escaping => SyntaxKind::Identifier,
        }
    }
}

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
    token.is_trivia()
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

        // A parenthesized type element, optionally prefixed by `mutating`
        // (only meaningful in function-type param position; stripped for
        // grouping/tuple). Yields `(Option<mutating-span>, TyVariant)`.
        //
        // ORDER IS LOAD-BEARING (plan D2): the FULL `ty` is attempted first,
        // because `ty` now admits kind prefixes. `mutating (T) -> R` is a
        // mutating-KIND function type; only when that fails do we fall back to
        // `mutating` as a per-param MutBorrow convention (`mutating T`). The
        // kind branch commits solely on the trailing `->`, so both readings
        // stay reachable and neither silently drops the keyword.
        let elem = ty
            .clone()
            .map(|t| (None, t))
            .or(skip_trivia()
                .ignore_then(just(Token::Mutating).map_with(|_, e| to_kestrel_span(e.span())))
                .map(Some)
                .then(ty.clone()))
            .boxed();

        // `'(' elems ')'` — the shared paren group behind both the
        // unit/grouping/tuple/function disambiguation below and the
        // kind-prefixed function type. Yields `(lparen, elems, has_comma, rparen)`.
        let paren_elems = skip_trivia()
            .ignore_then(just(Token::LParen).map_with(|_, e| to_kestrel_span(e.span())))
            .then(
                // Empty parens case
                skip_trivia()
                    .ignore_then(just(Token::RParen).map_with(|_, e| to_kestrel_span(e.span())))
                    .map(|rparen| (Vec::new(), false, rparen))
                    .or(
                        // At least one type
                        elem.clone()
                            .then(
                                // Check for comma after first element
                                skip_trivia()
                                    .ignore_then(
                                        just(Token::Comma)
                                            .map_with(|_, e| to_kestrel_span(e.span())),
                                    )
                                    .then(
                                        // After comma: more types separated by comma
                                        elem.clone()
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
            .map(|(lparen, (types, has_comma, rparen))| (lparen, types, has_comma, rparen))
            .boxed();

        // Kind-prefixed function type: `mutating (T) -> R`, `consuming (T) -> R`,
        // `escaping (T) -> R`. `escaping` is a CONTEXTUAL keyword (never
        // reserved); `mutating`/`consuming` are already hard keywords.
        //
        // The branch is fully backtrackable: it commits only once the `->`
        // after the closing paren is seen. Without the arrow the whole branch
        // rewinds, so `mutating (a, b)` still reads as a param convention on a
        // grouping/tuple in the contexts that allow one.
        let kind_prefix = skip_trivia()
            .ignore_then(
                just(Token::Mutating)
                    .map_with(|_, e| (FnKindPrefix::Mutating, to_kestrel_span(e.span())))
                    .or(just(Token::Consuming)
                        .map_with(|_, e| (FnKindPrefix::Consuming, to_kestrel_span(e.span()))))
                    .or(contextual_keyword("escaping").map(|span| (FnKindPrefix::Escaping, span))),
            )
            .boxed();

        let kinded_fn = kind_prefix
            .then(paren_elems.clone())
            .then(
                skip_trivia()
                    .ignore_then(just(Token::Arrow))
                    .map_with(|_, e| to_kestrel_span(e.span()))
                    .then(ty.clone()),
            )
            .map(
                |((kind, (lparen, types, _has_comma, rparen)), (arrow, ret))| {
                    TyVariant::Function(Some(kind), lparen, types, rparen, arrow, Box::new(ret))
                },
            )
            .boxed();

        // Unit type, grouping (T), tuple (T, U) or (T,), or function type
        // We need to distinguish:
        // - () -> Unit
        // - (T) -> Grouping (just returns T, for precedence)
        // - (T,) -> Single-element Tuple
        // - (T, U, ...) -> Tuple
        // - (...) -> T -> Function
        let paren_types = paren_elems
            .then(
                // Optional arrow and return type for function types
                skip_trivia()
                    .ignore_then(just(Token::Arrow))
                    .map_with(|_, e| to_kestrel_span(e.span()))
                    .then(ty.clone())
                    .or_not(),
            )
            .map(|((lparen, types, has_comma, rparen), arrow_and_return)| {
                if let Some((arrow_span, return_ty)) = arrow_and_return {
                    // Function type: keep per-param `mutating` markers.
                    TyVariant::Function(
                        None,
                        lparen,
                        types,
                        rparen,
                        arrow_span,
                        Box::new(return_ty),
                    )
                } else if types.is_empty() {
                    TyVariant::Unit(lparen, rparen)
                } else if types.len() == 1 && !has_comma {
                    // (T) - grouping, just return the inner type (drop marker)
                    types.into_iter().next().unwrap().1
                } else {
                    // (T,) or (T, U, ...) - tuple (markers not meaningful)
                    let elems = types.into_iter().map(|(_, t)| t).collect();
                    TyVariant::Tuple(lparen, elems, rparen)
                }
            })
            .boxed();

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

        // Opaque type: some P, some P and Q, some P and not Copyable
        // Each bound is a path type with optional type args
        let some_bound = path_segments_parser()
            .then(
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

        let some_type = skip_trivia()
            .ignore_then(just(Token::Some).map_with(|_, e| to_kestrel_span(e.span())))
            .then(
                some_bound
                    .clone()
                    .separated_by(
                        skip_trivia()
                            .ignore_then(just(Token::And))
                            .ignore_then(skip_trivia()),
                    )
                    .at_least(1)
                    .collect::<Vec<_>>(),
            )
            // Trailing negative bound: `and not Copyable`. The bounds list
            // above rewinds its trailing `and` when the next item starts with
            // `not`, letting this branch pick it up. Only `Copyable` is legal
            // as a negative — enforced at HIR lowering where the path resolves.
            .then(
                skip_trivia()
                    .ignore_then(just(Token::And))
                    .ignore_then(skip_trivia())
                    .ignore_then(just(Token::Not).map_with(|_, e| to_kestrel_span(e.span())))
                    .then_ignore(skip_trivia())
                    .then(some_bound)
                    .map(|(not_span, negative_ty)| (not_span, Box::new(negative_ty)))
                    .or_not(),
            )
            .map(|((some_span, bounds), negative)| TyVariant::Some(some_span, bounds, negative))
            .boxed();

        // Try some first (prefix keyword), then never, inferred, the
        // kind-prefixed function type (before `path`, so a contextual
        // `escaping` is not swallowed as a path segment), parens,
        // array/dict, path.
        let base_ty = some_type
            .or(never)
            .or(inferred)
            .or(kinded_fn)
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
            // `??` in type position is the double-optional sugar (`T?? == T??`):
            // the lexer greedily produces one `QuestionQuestion` token (it is the
            // nil-coalescing operator in expression position), so here we treat it
            // as two stacked `?` operators rather than a single one.
            DoubleOptional(Span),
            Throws(Span, TyVariant),
        }

        let type_operator = skip_trivia()
            .ignore_then(
                just(Token::Question)
                    .map_with(|_, e| TypeOperator::Optional(to_kestrel_span(e.span())))
                    .or(just(Token::QuestionQuestion)
                        .map_with(|_, e| TypeOperator::DoubleOptional(to_kestrel_span(e.span()))))
                    .or(just(Token::Throws)
                        .map_with(|_, e| to_kestrel_span(e.span()))
                        .then(ty.clone())
                        .map(|(throws_span, error_ty)| {
                            TypeOperator::Throws(throws_span, error_ty)
                        })),
            )
            .boxed();

        // Parse base type, then zero or more type operators
        let postfixed = base_ty
            .then(type_operator.repeated().collect::<Vec<_>>())
            .map(|(base, operators)| {
                // Apply operators left-to-right
                let mut result = base;
                for op in operators {
                    match op {
                        TypeOperator::Optional(question_span) => {
                            result = TyVariant::Optional(Box::new(result), question_span);
                        },
                        TypeOperator::DoubleOptional(question_span) => {
                            // `T??` -> Optional[Optional[T]]; both `?` share the
                            // `??` span. Chains like `T???` fall out naturally:
                            // the lexer yields `??` then `?`.
                            result = TyVariant::Optional(
                                Box::new(TyVariant::Optional(
                                    Box::new(result),
                                    question_span.clone(),
                                )),
                                question_span,
                            );
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
            .boxed();

        // Reference prefix: &T / &mutating T. Binds looser than the postfix
        // operators (`&T?` parses as `&(T?)`) and repeats so `&&T` parses
        // (`&&` lexes as two Ampersands — no compound token). Every ref
        // position is rejected at HIR lowering this stage; parsing anyway
        // buys real diagnostics + LSP recovery.
        let ref_prefix = skip_trivia()
            .ignore_then(just(Token::Ampersand).map_with(|_, e| to_kestrel_span(e.span())))
            .then(
                skip_trivia()
                    .ignore_then(just(Token::Mutating).map_with(|_, e| to_kestrel_span(e.span())))
                    .or_not(),
            );

        ref_prefix
            .repeated()
            .collect::<Vec<_>>()
            .then(postfixed)
            .map(|(prefixes, base)| {
                prefixes
                    .into_iter()
                    .rev()
                    .fold(base, |inner, (amp, mutating)| TyVariant::Ref {
                        amp,
                        mutating,
                        inner: Box::new(inner),
                    })
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
    use crate::parse_and_emit;

    parse_and_emit!(
        source,
        tokens,
        sink,
        ty_parser(),
        |sink, variant: TyVariant| emit_ty_variant(sink, &variant)
    );
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
        TyVariant::Function(kind, lparen, params, rparen, arrow, return_ty) => {
            emit_function_type(
                sink,
                kind.clone(),
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
        TyVariant::Some(some_span, bounds, negative) => {
            emit_some_type(sink, some_span.clone(), bounds, negative.as_ref());
        },
        TyVariant::Ref {
            amp,
            mutating,
            inner,
        } => {
            emit_ref_type(sink, amp.clone(), mutating.clone(), inner);
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
    /// Function type: `[kind] '(' params ')' '->' ret`.
    ///
    /// Fields in source order: the optional kind keyword prefix + its span,
    /// lparen, params, rparen, arrow, return type. Each param carries an
    /// optional `mutating` token span (`Some` ⇒ a `mutating` by-reference
    /// parameter) — a per-param convention, orthogonal to the kind.
    Function(
        Option<(FnKindPrefix, Span)>,
        Span,
        Vec<(Option<Span>, TyVariant)>,
        Span,
        Span,
        Box<TyVariant>,
    ),
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
    /// Opaque type: some P, some P and Q, some P and not Copyable
    /// (some_span, bound types, optional trailing negative bound: not_span + type)
    Some(Span, Vec<TyVariant>, Option<(Span, Box<TyVariant>)>),
    /// Reference type: &T or &mutating T. Parsed in every type position but
    /// accepted in none (stage 0.5) — rejection happens at HIR lowering.
    Ref {
        amp: Span,
        mutating: Option<Span>,
        inner: Box<TyVariant>,
    },
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
    kind: Option<(FnKindPrefix, Span)>,
    lparen: Span,
    params: &[(Option<Span>, TyVariant)],
    rparen: Span,
    arrow: Span,
    return_ty: &TyVariant,
) {
    sink.start_node(SyntaxKind::Ty);
    sink.start_node(SyntaxKind::TyFunction);

    // The kind keyword is a DIRECT child of TyFunction, before the TyList —
    // never inside it. The AST builder pairs bare `Mutating` tokens *inside*
    // TyList with the following param type by positional scan, so a kind
    // token placed there would be misread as a param convention.
    if let Some((prefix, span)) = kind {
        sink.add_token(prefix.syntax_kind(), span);
    }

    // Parameter list
    sink.start_node(SyntaxKind::TyList);
    sink.add_token(SyntaxKind::LParen, lparen);

    for (mutating, param) in params {
        // A `mutating` token sits in the TyList right before its param's Ty;
        // the AST builder scans for it positionally (Phase 3).
        if let Some(span) = mutating {
            sink.add_token(SyntaxKind::Mutating, span.clone());
        }
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

/// Emit events for a reference type: &T or &mutating T.
/// The node is atomic — the `mutating` token lives INSIDE TyRef/TyMutRef,
/// never as a sibling in a TyList, so the AST builder's positional
/// `mutating`-scan over function-type param lists cannot see it.
pub(crate) fn emit_ref_type(
    sink: &mut EventSink,
    amp: Span,
    mutating: Option<Span>,
    inner: &TyVariant,
) {
    sink.start_node(SyntaxKind::Ty);
    let kind = if mutating.is_some() {
        SyntaxKind::TyMutRef
    } else {
        SyntaxKind::TyRef
    };
    sink.start_node(kind);
    sink.add_token(SyntaxKind::Ampersand, amp);
    if let Some(mut_span) = mutating {
        sink.add_token(SyntaxKind::Mutating, mut_span);
    }
    emit_ty_variant(sink, inner);
    sink.finish_node(); // Finish TyRef/TyMutRef
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

pub(crate) fn emit_some_type(
    sink: &mut EventSink,
    some_span: Span,
    bounds: &[TyVariant],
    negative: Option<&(Span, Box<TyVariant>)>,
) {
    sink.start_node(SyntaxKind::Ty);
    sink.start_node(SyntaxKind::TySome);

    sink.add_token(SyntaxKind::Some, some_span);
    for (i, bound) in bounds.iter().enumerate() {
        if i > 0 {
            // `and` tokens between bounds aren't tracked as separate spans
            // since they're consumed during parsing; emit the bound type directly
        }
        emit_ty_variant(sink, bound);
    }

    // Negative bound (`and not Copyable`) uses the same NegativeConformance
    // wrapper as conformance lists, so the ast-builder can tell it apart from
    // the positive bounds (which are direct type-node children).
    if let Some((not_span, negative_ty)) = negative {
        sink.start_node(SyntaxKind::NegativeConformance);
        sink.add_token(SyntaxKind::Not, not_span.clone());
        emit_ty_variant(sink, negative_ty);
        sink.finish_node();
    }

    sink.finish_node(); // Finish TySome
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

    /// The first `TyFunction` node in the tree.
    fn first_fn_node(ty: &TyExpression) -> SyntaxNode {
        ty.syntax
            .descendants()
            .find(|n| n.kind() == SyntaxKind::TyFunction)
            .expect("expected a TyFunction node")
    }

    /// Text of the kind keyword emitted as a DIRECT child of `TyFunction`
    /// before its `TyList` (`None` for the unmarked normal kind).
    fn fn_kind_text(func: &SyntaxNode) -> Option<String> {
        for child in func.children_with_tokens() {
            if child
                .as_node()
                .is_some_and(|n| n.kind() == SyntaxKind::TyList)
            {
                break;
            }
            if let Some(tok) = child.as_token()
                && matches!(
                    tok.kind(),
                    SyntaxKind::Mutating | SyntaxKind::Consuming | SyntaxKind::Identifier
                )
            {
                return Some(tok.text().to_string());
            }
        }
        None
    }

    /// Does the function's own `TyList` carry a bare `mutating` token — i.e.
    /// a per-param MutBorrow convention?
    fn has_param_mutating(func: &SyntaxNode) -> bool {
        func.children()
            .find(|n| n.kind() == SyntaxKind::TyList)
            .is_some_and(|list| {
                list.children_with_tokens()
                    .filter_map(|c| c.into_token())
                    .any(|t| t.kind() == SyntaxKind::Mutating)
            })
    }

    #[test]
    fn test_fn_type_kind_prefixes() {
        for (source, keyword) in [
            ("mutating () -> Int", "mutating"),
            ("consuming (Int) -> Bool", "consuming"),
            ("escaping () -> Int", "escaping"),
        ] {
            let ty = parse_ty_from_source(source);
            assert!(ty.is_function(), "{source} should be a function type");
            let func = first_fn_node(&ty);
            assert_eq!(
                fn_kind_text(&func).as_deref(),
                Some(keyword),
                "{source} should carry its kind keyword on TyFunction"
            );
            assert!(
                !has_param_mutating(&func),
                "{source}: kind keyword must not land inside TyList"
            );
        }
    }

    #[test]
    fn test_plain_fn_type_has_no_kind() {
        let ty = parse_ty_from_source("(Int) -> Bool");
        assert_eq!(fn_kind_text(&first_fn_node(&ty)), None);
    }

    /// Reading 1 (plan D2): `(mutating () -> ())` is a GROUPED mutating-kind
    /// function type — parenthesising must not drop the kind.
    #[test]
    fn test_grouped_kinded_fn_type_keeps_kind() {
        let ty = parse_ty_from_source("(mutating () -> ())");
        assert!(
            ty.is_function(),
            "the grouping should unwrap to the fn type"
        );
        let func = first_fn_node(&ty);
        assert_eq!(fn_kind_text(&func).as_deref(), Some("mutating"));
    }

    /// Reading 2 (plan D2): in `(mutating (T) -> R) -> U` the PARAM is a
    /// mutating-kind function type, not a MutBorrow-convention param of a
    /// normal one.
    #[test]
    fn test_kinded_fn_type_as_param() {
        let ty = parse_ty_from_source("(mutating (T) -> R) -> U");
        let mut fns = ty
            .syntax
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::TyFunction);
        let outer = fns.next().expect("outer function type");
        let inner = fns.next().expect("inner (param) function type");
        assert_eq!(fn_kind_text(&outer), None, "outer type is normal-kind");
        assert!(
            !has_param_mutating(&outer),
            "the keyword belongs to the param's type, not to a convention"
        );
        assert_eq!(fn_kind_text(&inner).as_deref(), Some("mutating"));
    }

    /// Reading 3 (plan D2): the same shape with the contextual `escaping`.
    #[test]
    fn test_escaping_fn_type_as_param() {
        let ty = parse_ty_from_source("(escaping () -> ()) -> U");
        let mut fns = ty
            .syntax
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::TyFunction);
        let outer = fns.next().expect("outer function type");
        let inner = fns.next().expect("inner (param) function type");
        assert_eq!(fn_kind_text(&outer), None);
        assert_eq!(fn_kind_text(&inner).as_deref(), Some("escaping"));
    }

    /// The `mutating` PARAM CONVENTION is unchanged: no trailing `->` after
    /// the paren group means the kind branch backtracks.
    #[test]
    fn test_param_convention_still_parses() {
        let ty = parse_ty_from_source("(mutating Int64) -> ()");
        let func = first_fn_node(&ty);
        assert_eq!(fn_kind_text(&func), None, "no whole-type kind here");
        assert!(has_param_mutating(&func), "expected a MutBorrow param");
    }

    /// `mutating (T)` is a MutBorrow convention on a grouped `T` — the kind
    /// branch requires the arrow and rewinds without it.
    #[test]
    fn test_kind_branch_backtracks_without_arrow() {
        let ty = parse_ty_from_source("(mutating (Int64)) -> ()");
        let func = first_fn_node(&ty);
        assert_eq!(fn_kind_text(&func), None);
        assert!(has_param_mutating(&func));
        assert_eq!(
            func.descendants()
                .filter(|n| n.kind() == SyntaxKind::TyFunction)
                .count(),
            1,
            "the grouped `(Int64)` must not become a function type"
        );
    }

    /// `escaping` stays an ordinary identifier: as a bare type name it is a
    /// path, not a dropped keyword.
    #[test]
    fn test_escaping_remains_an_identifier() {
        let ty = parse_ty_from_source("escaping");
        assert!(ty.is_path());
        assert_eq!(ty.path_segments(), Some(vec!["escaping".to_string()]));
    }

    #[test]
    fn test_some_type_basic() {
        let source = "some Shape";
        let ty = parse_ty_from_source(source);

        let some_node = ty
            .syntax
            .descendants()
            .find(|n| n.kind() == SyntaxKind::TySome)
            .expect("expected a TySome node");
        assert!(
            !some_node
                .descendants()
                .any(|n| n.kind() == SyntaxKind::NegativeConformance),
            "plain `some P` must not carry a negative bound"
        );
    }

    #[test]
    fn test_some_type_with_negative_bound() {
        let source = "some Shape and Equatable and not Copyable";
        let ty = parse_ty_from_source(source);

        let some_node = ty
            .syntax
            .descendants()
            .find(|n| n.kind() == SyntaxKind::TySome)
            .expect("expected a TySome node");

        // Positive bounds are direct type-node children; the negative bound
        // lives inside a NegativeConformance wrapper.
        let direct_ty_children = some_node
            .children()
            .filter(|c| c.kind() == SyntaxKind::Ty)
            .count();
        assert_eq!(direct_ty_children, 2, "two positive bounds expected");

        let negative = some_node
            .children()
            .find(|c| c.kind() == SyntaxKind::NegativeConformance)
            .expect("expected a NegativeConformance child for `not Copyable`");
        assert!(
            negative.descendants().any(|n| n.kind() == SyntaxKind::Ty),
            "negative bound should wrap a type node"
        );
    }
}
