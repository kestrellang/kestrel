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
use kestrel_lexer2::Token;
use kestrel_span2::Span;
use kestrel_syntax_tree2::{SyntaxKind, SyntaxNode};

use crate::attribute::attribute_list_parser;
use crate::common::{
    AssociatedTypeBoundsData, AssociatedTypeTargetData, TypeAliasDeclarationData,
    emit_type_alias_declaration, identifier, token, visibility_parser_internal,
};
use crate::event::{EventSink, TreeBuilder};
use crate::input::{ParserExtra, ParserInput, create_input, prepare_tokens};
use crate::ty::{TyVariant, ty_parser};
use crate::type_param::{type_parameter_list_parser, where_clause_parser};

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
    let prepared = prepare_tokens(tokens);
    let input = create_input(&prepared, source.len());

    match type_alias_declaration_parser_internal()
        .parse(input)
        .into_result()
    {
        Ok(data) => {
            emit_type_alias_declaration(sink, data);
        },
        Err(errors) => {
            for error in errors {
                sink.error_from_rich(&error);
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_lexer2::lex;

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
