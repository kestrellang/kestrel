//! Unified type declaration parser for structs and enums
//!
//! This module provides a combined parser that handles both struct and enum
//! declarations in a single recursive context. This is necessary because
//! structs can contain nested enums and enums can contain nested structs,
//! creating mutual recursion that would otherwise cause stack overflow
//! with separate recursive parsers.
//!
//! The key insight is that by using a single `recursive()` call that handles
//! both types, the parser shares a single recursion context, dramatically
//! reducing stack usage for deeply nested type declarations.

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;

use crate::attribute::attribute_list_parser;
use crate::common::{
    ConformanceListData, EnumCaseDeclarationData, EnumCaseParameterData, EnumDeclarationData,
    StructDeclarationData, TypeDeclarationBodyItem, deinit_declaration_parser_internal,
    field_declaration_parser_internal, function_declaration_parser_internal, identifier,
    import_declaration_parser_internal, initializer_declaration_parser_internal,
    module_declaration_parser_internal, token, visibility_parser_internal,
};
use crate::input::{ParserExtra, ParserInput, to_kestrel_span};
use crate::ty::ty_parser;
use crate::type_alias::type_alias_declaration_parser_internal;
use crate::type_param::{conformance_list_parser, type_parameter_list_parser, where_clause_parser};

/// A unified type declaration - either a struct or an enum
#[derive(Debug, Clone)]
pub enum TypeDeclarationData {
    Struct(StructDeclarationData),
    Enum(EnumDeclarationData),
}

/// Parser for enum case parameter: `label: Type` or just `Type`
///
/// Supports both:
/// - Named: `label: Type` (e.g., `value: Int`)
/// - Unnamed: `Type` (e.g., `Int` or `T`)
fn enum_case_parameter_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, EnumCaseParameterData, ParserExtra<'tokens>> + Clone {
    // Try named form first: `identifier: Type`
    let named = identifier()
        .then(token(Token::Colon))
        .then(ty_parser())
        .map(|((label, colon), ty)| EnumCaseParameterData {
            label: Some(label),
            colon: Some(colon),
            ty,
        });

    // Unnamed form: just `Type`
    let unnamed = ty_parser().map(|ty| EnumCaseParameterData {
        label: None,
        colon: None,
        ty,
    });

    // Try named first, fall back to unnamed
    named.or(unnamed)
}

/// Parser for enum case declaration: `(@attr)* case Name` or `(@attr)* case Name(label: Type, ...)`
fn enum_case_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, EnumCaseDeclarationData, ParserExtra<'tokens>> + Clone
{
    attribute_list_parser()
        .then(token(Token::Case))
        .then(identifier())
        .then(
            // Optional parameter list: (label: Type, label: Type, ...)
            token(Token::LParen)
                .then(
                    enum_case_parameter_parser()
                        .separated_by(just(Token::Comma))
                        .allow_trailing()
                        .collect::<Vec<_>>(),
                )
                .then(token(Token::RParen))
                .map(|((lparen, params), rparen)| Some((lparen, params, rparen)))
                .or(empty().map(|_| None)),
        )
        .map(
            |(((attributes, case_span), name_span), parameters)| EnumCaseDeclarationData {
                attributes,
                case_span,
                name_span,
                parameters,
            },
        )
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

/// Parser for the optional `indirect` modifier
fn indirect_modifier_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, Option<Span>, ParserExtra<'tokens>> + Clone {
    skip_trivia()
        .ignore_then(just(Token::Indirect).map_with(|_, e| Some(to_kestrel_span(e.span()))))
        .or(empty().to(None))
}

/// Internal parser for type body items (used by both struct and enum)
///
/// The `type_parser` parameter is a unified parser that can parse either struct or enum,
/// allowing mutual nesting without separate recursive contexts.
fn type_body_item_parser_internal<'tokens>(
    type_parser: impl Parser<'tokens, ParserInput<'tokens>, TypeDeclarationData, ParserExtra<'tokens>>
    + Clone
    + 'tokens,
    is_enum: bool,
) -> impl Parser<'tokens, ParserInput<'tokens>, TypeDeclarationBodyItem, ParserExtra<'tokens>> + Clone
{
    let module_parser = module_declaration_parser_internal()
        .map(|(module_span, path)| TypeDeclarationBodyItem::Module(module_span, path));

    let import_parser =
        import_declaration_parser_internal().map(|(import_span, path, alias, items)| {
            TypeDeclarationBodyItem::Import(import_span, path, alias, items)
        });

    // Use the unified type parser - it returns TypeDeclarationData which we need to split
    let nested_type_parser = type_parser.map(|data| match data {
        TypeDeclarationData::Struct(s) => TypeDeclarationBodyItem::Struct(Box::new(s)),
        TypeDeclarationData::Enum(e) => TypeDeclarationBodyItem::Enum(Box::new(e)),
    });

    let initializer_parser =
        initializer_declaration_parser_internal().map(TypeDeclarationBodyItem::Initializer);

    let deinit_parser = deinit_declaration_parser_internal().map(TypeDeclarationBodyItem::Deinit);

    let function_parser =
        function_declaration_parser_internal().map(TypeDeclarationBodyItem::Function);

    let type_alias_parser =
        type_alias_declaration_parser_internal().map(TypeDeclarationBodyItem::TypeAlias);

    let field_parser = field_declaration_parser_internal().map(TypeDeclarationBodyItem::Field);

    // Enum case parser (only for enums)
    let case_parser = enum_case_parser().map(TypeDeclarationBodyItem::EnumCase);

    if is_enum {
        // For enums: cases first, then shared items
        module_parser
            .or(import_parser)
            .or(case_parser)
            .or(nested_type_parser)
            .or(initializer_parser)
            .or(type_alias_parser)
            .or(function_parser)
            .or(field_parser)
            .boxed()
    } else {
        // For structs: no cases, include deinit
        module_parser
            .or(import_parser)
            .or(nested_type_parser)
            .or(initializer_parser)
            .or(deinit_parser)
            .or(type_alias_parser)
            .or(function_parser)
            .or(field_parser)
            .boxed()
    }
}

/// Unified parser for both struct and enum declarations
///
/// This uses a single `recursive()` call to handle both struct and enum,
/// which allows mutual nesting (struct containing enum containing struct, etc.)
/// without creating separate recursive parser contexts that would cause
/// stack overflow on deeply nested types.
pub fn type_declaration_parser_internal<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, TypeDeclarationData, ParserExtra<'tokens>> + Clone {
    recursive(|type_parser| {
        // Box the recursive reference to reduce stack frame size
        let type_parser = type_parser.boxed();

        // Struct parser - use boxed() to reduce stack frame size
        let struct_body_parser = type_body_item_parser_internal(type_parser.clone(), false)
            .repeated()
            .collect::<Vec<_>>()
            .boxed();

        let struct_parser = attribute_list_parser()
            .then(visibility_parser_internal())
            .then(token(Token::Struct))
            .then(identifier())
            .then(type_parameter_list_parser().or_not())
            .then(conformance_list_parser().or_not())
            .then(where_clause_parser().or_not())
            .then(token(Token::LBrace))
            .then(struct_body_parser)
            .then(token(Token::RBrace))
            .map(
                |(
                    (
                        (
                            (
                                (
                                    (
                                        (((attributes, visibility), struct_span), name_span),
                                        type_params,
                                    ),
                                    conformances,
                                ),
                                where_clause,
                            ),
                            lbrace_span,
                        ),
                        body,
                    ),
                    rbrace_span,
                )| {
                    TypeDeclarationData::Struct(StructDeclarationData {
                        attributes,
                        visibility,
                        struct_span,
                        name_span,
                        type_params,
                        conformances: conformances.map(|(colon_span, items)| ConformanceListData {
                            colon_span,
                            conformances: items,
                        }),
                        where_clause,
                        lbrace_span,
                        body,
                        rbrace_span,
                    })
                },
            )
            .boxed();

        // Enum parser - use boxed() to reduce stack frame size
        let enum_body_parser = type_body_item_parser_internal(type_parser, true)
            .repeated()
            .collect::<Vec<_>>()
            .boxed();

        let enum_parser = attribute_list_parser()
            .then(visibility_parser_internal())
            .then(indirect_modifier_parser())
            .then(token(Token::Enum))
            .then(identifier())
            .then(type_parameter_list_parser().or_not())
            .then(conformance_list_parser().or_not())
            .then(where_clause_parser().or_not())
            .then(token(Token::LBrace))
            .then(enum_body_parser)
            .then(token(Token::RBrace))
            .map(
                |(
                    (
                        (
                            (
                                (
                                    (
                                        (
                                            (((attributes, visibility), indirect), enum_span),
                                            name_span,
                                        ),
                                        type_params,
                                    ),
                                    conformances,
                                ),
                                where_clause,
                            ),
                            lbrace_span,
                        ),
                        body,
                    ),
                    rbrace_span,
                )| {
                    TypeDeclarationData::Enum(EnumDeclarationData {
                        attributes,
                        visibility,
                        indirect,
                        enum_span,
                        name_span,
                        type_params,
                        conformances: conformances.map(|(colon_span, items)| ConformanceListData {
                            colon_span,
                            conformances: items,
                        }),
                        where_clause,
                        lbrace_span,
                        body,
                        rbrace_span,
                    })
                },
            )
            .boxed();

        // Try struct first, then enum
        struct_parser.or(enum_parser)
    })
}

/// Parser that only returns struct declarations (filters out enums)
pub fn struct_declaration_parser_unified<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, StructDeclarationData, ParserExtra<'tokens>> + Clone {
    type_declaration_parser_internal().try_map(|data, span| match data {
        TypeDeclarationData::Struct(s) => Ok(s),
        TypeDeclarationData::Enum(_) => Err(Rich::custom(span, "Expected struct, found enum")),
    })
}

/// Parser that only returns enum declarations (filters out structs)
pub fn enum_declaration_parser_unified<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, EnumDeclarationData, ParserExtra<'tokens>> + Clone {
    type_declaration_parser_internal().try_map(|data, span| match data {
        TypeDeclarationData::Enum(e) => Ok(e),
        TypeDeclarationData::Struct(_) => Err(Rich::custom(span, "Expected enum, found struct")),
    })
}
