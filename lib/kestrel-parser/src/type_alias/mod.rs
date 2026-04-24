//! Type alias declaration parsing
//!
//! This module is the single source of truth for type alias declaration parsing.
//! Type aliases support generics with type parameters.
//!
//! # Supported Syntax
//!
//! ## Regular type aliases (module/file level)
//! ```text
//! type Alias = Type;
//! type Box[T] = T;
//! public type MyInt = Int;
//! ```
//!
//! ## Associated types in protocols
//! ```text
//! type Item                        // Abstract associated type
//! type Item: Equatable             // With constraint bounds
//! type Item = Int                  // With default
//! type Item: Equatable = Int       // With bounds and default
//! ```
//!
//! ## Associated type bindings in structs
//! ```text
//! type Item = Int                  // Simple binding
//! type Iterator.Item = Int         // Qualified (multiple conformances)
//! type Add[Int].Output = Int       // Qualified with type arguments
//! ```

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::attribute::attribute_list_parser;
use crate::common::{
    AttributeData, emit_attribute_list, emit_name, emit_visibility, identifier, token,
    visibility_parser_internal,
};
use crate::event::{EventSink, TreeBuilder};
use crate::input::{ParserExtra, ParserInput};
use crate::parse_and_emit;
use crate::ty::{TyVariant, emit_ty_variant, ty_parser};
use crate::type_param::{
    TypeParameterData, WhereClauseData, emit_type_parameter_list, emit_where_clause,
    type_parameter_list_parser, where_clause_parser,
};

/// Represents a type alias declaration: (visibility)? type Name[T]? = Type;
///
/// The declaration is stored as a lossless syntax tree. All data is derived
/// from the tree rather than stored separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeAliasDeclaration {
    pub syntax: SyntaxNode,
    pub span: Span,
}

impl TypeAliasDeclaration {
    /// Create a new TypeAliasDeclaration from events and source text
    pub fn from_events(source: &str, events: Vec<crate::event::Event>, span: Span) -> Self {
        let builder = TreeBuilder::new(source, events);
        let syntax = builder.build();
        Self { syntax, span }
    }

    /// Get the alias name from this declaration
    pub fn name(&self) -> Option<String> {
        self.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::Name)?
            .children_with_tokens()
            .filter_map(|elem| elem.into_token())
            .find(|tok| tok.kind() == SyntaxKind::Identifier)
            .map(|tok| tok.text().to_string())
    }

    /// Get the visibility modifier if present
    pub fn visibility(&self) -> Option<SyntaxKind> {
        let visibility_node = self
            .syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::Visibility)?;

        visibility_node
            .children_with_tokens()
            .filter_map(|elem| elem.into_token())
            .find(|tok| {
                matches!(
                    tok.kind(),
                    SyntaxKind::Public
                        | SyntaxKind::Private
                        | SyntaxKind::Internal
                        | SyntaxKind::Fileprivate
                )
            })
            .map(|tok| tok.kind())
    }

    /// Check if this type alias has type parameters
    pub fn has_type_parameters(&self) -> bool {
        self.syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::TypeParameterList)
    }

    /// Get the aliased type name from this declaration
    /// Note: This is a best-effort helper for simple path types.
    pub fn aliased_type(&self) -> Option<String> {
        let aliased_node = self
            .syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::AliasedType)?;

        let ty_node = aliased_node
            .children()
            .find(|child| child.kind() == SyntaxKind::Ty)?;

        let ty_path_node = ty_node
            .children()
            .find(|child| child.kind() == SyntaxKind::TyPath)?;

        let path_node = ty_path_node
            .children()
            .find(|child| child.kind() == SyntaxKind::Path)?;

        let segments: Vec<String> = path_node
            .children()
            .filter(|child| child.kind() == SyntaxKind::PathElement)
            .filter_map(|elem| {
                elem.children_with_tokens()
                    .filter_map(|t| t.into_token())
                    .find(|tok| tok.kind() == SyntaxKind::Identifier)
                    .map(|tok| tok.text().to_string())
            })
            .collect();

        if segments.is_empty() {
            None
        } else {
            Some(segments.join("."))
        }
    }
}

/// Raw parsed data for type alias declaration internals.
#[derive(Debug, Clone)]
pub struct TypeAliasDeclarationData {
    pub attributes: Vec<AttributeData>,
    pub visibility: Option<(Token, Span)>,
    pub type_span: Span,
    /// The target of the type alias - simple name or qualified path.
    pub target: AssociatedTypeTargetData,
    pub type_params: Option<(Span, Vec<TypeParameterData>, Span)>,
    /// Optional bounds for associated types (: Equatable, Hashable).
    pub bounds: Option<AssociatedTypeBoundsData>,
    /// Optional where clause for associated types (where Iter.Item = Item).
    pub where_clause: Option<WhereClauseData>,
    /// Optional equals span and aliased type (= Type).
    /// For associated types in protocols, this may be None (abstract associated type).
    pub aliased: Option<(Span, TyVariant)>,
    pub semicolon_span: Option<Span>,
}

/// Target for type alias - either simple name or qualified path.
#[derive(Debug, Clone)]
pub enum AssociatedTypeTargetData {
    /// Simple name: `type Item`.
    Simple(Span),
    /// Qualified path: `type Iterator.Item` or `type Add[Int].Output`.
    Qualified {
        /// The protocol path (may include type arguments).
        protocol_path: TyVariant,
        /// The dot before the name.
        dot_span: Span,
        /// The associated type name.
        name_span: Span,
    },
}

/// Bounds for associated types (: Equatable, Hashable).
#[derive(Debug, Clone)]
pub struct AssociatedTypeBoundsData {
    pub colon_span: Span,
    /// The bound types (protocols).
    pub bounds: Vec<TyVariant>,
}

/// Parser for associated type bounds (: Equatable, Hashable)
fn associated_type_bounds_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, AssociatedTypeBoundsData, ParserExtra<'tokens>> + Clone
{
    token(Token::Colon)
        .then(
            ty_parser()
                .separated_by(just(Token::Comma))
                .at_least(1)
                .collect(),
        )
        .map(|(colon_span, bounds)| AssociatedTypeBoundsData { colon_span, bounds })
        .boxed()
}

/// Parser for associated type target (simple name or qualified path)
///
/// Supports:
/// - Simple: `Item`
/// - Qualified: `Iterator.Item` or `Add[Int].Output`
///
/// Strategy: Parse an identifier first, then check if there's more (dot + name).
/// If we see a dot, we need to determine if it's part of a type path or the
/// associated type accessor. The key insight is that the qualified form always
/// ends with `.Name` where Name is a simple identifier.
fn associated_type_target_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, AssociatedTypeTargetData, ParserExtra<'tokens>> + Clone
{
    // Simple approach: parse identifier, optionally followed by more path segments
    // and a final .name
    //
    // For now, support:
    // - Simple: identifier
    // - Qualified: identifier.identifier (one level)
    // - Qualified with generics: identifier[types].identifier
    //
    // We use try_map to collect all segments and separate the last one
    identifier()
        .then(
            // Optional: generic args followed by dot and name
            // Or just: dot and name
            just(Token::LBracket)
                .ignore_then(
                    ty_parser()
                        .separated_by(just(Token::Comma))
                        .at_least(1)
                        .collect(),
                )
                .then_ignore(just(Token::RBracket))
                .or_not()
                .then(token(Token::Dot))
                .then(identifier())
                .or_not(),
        )
        .map(|(first_name, rest)| {
            match rest {
                None => {
                    // Simple case: just an identifier
                    AssociatedTypeTargetData::Simple(first_name)
                },
                Some(((type_args, dot_span), name_span)) => {
                    // Qualified case: first_name[type_args]?.name
                    // Reconstruct protocol_path as a TyVariant::Path
                    let protocol_path = TyVariant::Path {
                        segments: vec![first_name.clone()],
                        args: type_args,
                    };
                    AssociatedTypeTargetData::Qualified {
                        protocol_path,
                        dot_span,
                        name_span,
                    }
                },
            }
        })
        .boxed()
}

/// Internal Chumsky parser for type alias declaration
///
/// This is the single source of truth for type alias declaration parsing.
/// Supports all variants:
/// - Regular: `type Alias = Type;`
/// - Associated type (protocol): `type Item;` or `type Item: Bound;` or `type Item = Default;`
/// - Qualified binding (struct): `type Iterator.Item = Int;`
pub fn type_alias_declaration_parser_internal<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, TypeAliasDeclarationData, ParserExtra<'tokens>> + Clone
{
    attribute_list_parser()
        .then(visibility_parser_internal())
        .then(token(Token::Type))
        .then(associated_type_target_parser())
        .then(type_parameter_list_parser().or_not())
        .then(associated_type_bounds_parser().or_not())
        .then(where_clause_parser().or_not())
        .then(token(Token::Equals).then(ty_parser()).or_not())
        .then(token(Token::Semicolon).or_not())
        .map(
            |(
                (
                    (
                        (((((attributes, visibility), type_span), target), type_params), bounds),
                        where_clause,
                    ),
                    aliased,
                ),
                semicolon_span,
            )| {
                TypeAliasDeclarationData {
                    attributes,
                    visibility,
                    type_span,
                    target,
                    type_params,
                    bounds,
                    where_clause,
                    aliased,
                    semicolon_span,
                }
            },
        )
        .boxed()
}

/// Parse a type alias declaration and emit events
///
/// This is the primary event-driven parser function for type alias declarations.
pub fn parse_type_alias_declaration<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    parse_and_emit!(
        source,
        tokens,
        sink,
        type_alias_declaration_parser_internal(),
        emit_type_alias_declaration
    );
}

/// Emit events for an associated type target.
fn emit_associated_type_target(sink: &mut EventSink, target: &AssociatedTypeTargetData) {
    match target {
        AssociatedTypeTargetData::Simple(name_span) => {
            emit_name(sink, name_span.clone());
        },
        AssociatedTypeTargetData::Qualified {
            protocol_path,
            dot_span,
            name_span,
        } => {
            sink.start_node(SyntaxKind::AssociatedTypeTarget);
            emit_ty_variant(sink, protocol_path);
            sink.add_token(SyntaxKind::Dot, dot_span.clone());
            emit_name(sink, name_span.clone());
            sink.finish_node();
        },
    }
}

/// Emit events for associated type bounds (: Equatable, Hashable).
fn emit_associated_type_bounds(sink: &mut EventSink, bounds: &AssociatedTypeBoundsData) {
    sink.start_node(SyntaxKind::ConformanceList);
    sink.add_token(SyntaxKind::Colon, bounds.colon_span.clone());
    for (i, bound) in bounds.bounds.iter().enumerate() {
        if i > 0 {
            let prev_end = bounds.colon_span.end + i;
            sink.add_token(
                SyntaxKind::Comma,
                Span::new(bounds.colon_span.file_id, prev_end..prev_end + 1),
            );
        }
        sink.start_node(SyntaxKind::ConformanceItem);
        emit_ty_variant(sink, bound);
        sink.finish_node();
    }
    sink.finish_node();
}

/// Emit events for a type alias declaration.
///
/// Destructures `TypeAliasDeclarationData` without a `..` rest pattern:
/// adding a field forces this function to stop compiling until the new
/// field is handled in emission.
pub(crate) fn emit_type_alias_declaration(
    sink: &mut EventSink,
    data: TypeAliasDeclarationData,
) {
    let TypeAliasDeclarationData {
        attributes,
        visibility,
        type_span,
        target,
        type_params,
        bounds,
        where_clause,
        aliased,
        semicolon_span,
    } = data;

    sink.start_node(SyntaxKind::TypeAliasDeclaration);

    emit_attribute_list(sink, &attributes);
    emit_visibility(sink, visibility);
    sink.add_token(SyntaxKind::Type, type_span);

    emit_associated_type_target(sink, &target);

    if let Some((lbracket, params, rbracket)) = type_params {
        emit_type_parameter_list(sink, lbracket, params, rbracket);
    }

    if let Some(ref bounds) = bounds {
        emit_associated_type_bounds(sink, bounds);
    }

    if let Some(wc) = where_clause {
        emit_where_clause(sink, wc);
    }

    if let Some((equals_span, ref aliased_type)) = aliased {
        sink.add_token(SyntaxKind::Equals, equals_span);
        sink.start_node(SyntaxKind::AliasedType);
        emit_ty_variant(sink, aliased_type);
        sink.finish_node();
    }

    if let Some(semicolon_span) = semicolon_span {
        sink.add_token(SyntaxKind::Semicolon, semicolon_span);
    }

    sink.finish_node();
}

impl crate::event::EmitSyntax for TypeAliasDeclarationData {
    fn emit(self, sink: &mut EventSink) {
        emit_type_alias_declaration(sink, self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_lexer::lex;

    #[test]
    fn test_type_alias_declaration_basic() {
        let source = "type Alias = Aliased;";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_type_alias_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = TypeAliasDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("Alias".to_string()));
        assert_eq!(decl.aliased_type(), Some("Aliased".to_string()));
        assert_eq!(decl.visibility(), None);
    }

    #[test]
    fn test_type_alias_declaration_with_visibility() {
        let source = "public type PublicAlias = SomeType;";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_type_alias_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = TypeAliasDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("PublicAlias".to_string()));
        assert_eq!(decl.visibility(), Some(SyntaxKind::Public));
    }

    #[test]
    fn test_type_alias_declaration_with_generics() {
        let source = "type Box[T] = T;";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_type_alias_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = TypeAliasDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("Box".to_string()));
        assert!(decl.has_type_parameters());
    }

    #[test]
    fn test_type_alias_abstract_associated_type() {
        // Associated type in protocol without default: type Item;
        let source = "type Item;";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_type_alias_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = TypeAliasDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("Item".to_string()));
        // No aliased type for abstract associated types
        assert_eq!(decl.aliased_type(), None);
        // No AliasedType node should exist
        assert!(
            !decl
                .syntax
                .children()
                .any(|c| c.kind() == SyntaxKind::AliasedType)
        );
    }

    #[test]
    fn test_type_alias_declaration_tuple() {
        let source = "type TupleAlias = (Int, String);";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_type_alias_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = TypeAliasDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("TupleAlias".to_string()));
        // aliased_type() only works for simple paths
        assert_eq!(decl.aliased_type(), None);

        // Verify structure
        let aliased = decl
            .syntax
            .children()
            .find(|c| c.kind() == SyntaxKind::AliasedType)
            .unwrap();
        let ty = aliased
            .children()
            .find(|c| c.kind() == SyntaxKind::Ty)
            .unwrap();
        assert_eq!(ty.children().next().unwrap().kind(), SyntaxKind::TyTuple);
    }
}
