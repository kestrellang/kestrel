//! Coordinator for the mutually-recursive struct/enum declaration parsers.
//!
//! Structs can nest enums and enums can nest structs, so a single
//! `recursive()` call is used to share one recursion context between the two
//! grammars. Creating independent recursive parsers per type would double the
//! stack usage on deeply nested type declarations.
//!
//! The body and header grammars themselves live in the per-type modules:
//!
//! - `struct::struct_parser_with_recursion` — struct header + body
//! - `enum_decl::enum_parser_with_recursion` — enum header + body + cases
//!
//! Each factory accepts the shared `type_parser` handle as a generic argument
//! and returns the type-specific data shape. This module only owns the
//! `TypeDeclarationData` union, the `recursive()` glue, and the two
//! variant-filtering wrappers exposed to the per-type modules.

use chumsky::prelude::*;

use crate::enum_decl::{EnumDeclarationData, enum_parser_with_recursion};
use crate::input::{ParserExtra, ParserInput};
use crate::r#struct::{StructDeclarationData, struct_parser_with_recursion};

/// A unified type declaration — either a struct or an enum.
#[derive(Debug, Clone)]
pub enum TypeDeclarationData {
    Struct(StructDeclarationData),
    Enum(EnumDeclarationData),
}

/// Unified parser for struct and enum declarations.
///
/// Shares a single `recursive()` context between the two so a struct nested
/// inside an enum (or vice versa) does not open a new recursive parser.
pub fn type_declaration_parser_internal<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, TypeDeclarationData, ParserExtra<'tokens>> + Clone {
    recursive(|type_parser| {
        let type_parser = type_parser.boxed();

        let struct_parser =
            struct_parser_with_recursion(type_parser.clone()).map(TypeDeclarationData::Struct);
        let enum_parser = enum_parser_with_recursion(type_parser).map(TypeDeclarationData::Enum);

        struct_parser.or(enum_parser)
    })
}

/// Parser that only returns struct declarations (filters out enums).
pub fn struct_declaration_parser_unified<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, StructDeclarationData, ParserExtra<'tokens>> + Clone {
    type_declaration_parser_internal().try_map(|data, span| match data {
        TypeDeclarationData::Struct(s) => Ok(s),
        TypeDeclarationData::Enum(_) => Err(Rich::custom(span, "Expected struct, found enum")),
    })
}

/// Parser that only returns enum declarations (filters out structs).
pub fn enum_declaration_parser_unified<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, EnumDeclarationData, ParserExtra<'tokens>> + Clone {
    type_declaration_parser_internal().try_map(|data, span| match data {
        TypeDeclarationData::Enum(e) => Ok(e),
        TypeDeclarationData::Struct(_) => Err(Rich::custom(span, "Expected enum, found struct")),
    })
}
