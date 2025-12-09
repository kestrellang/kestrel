//! Struct declaration parsing
//!
//! This module is the single source of truth for struct declaration parsing.
//! Struct bodies can contain: fields, functions, nested structs, modules, and imports.

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::event::{EventSink, TreeBuilder};
use crate::common::{
    visibility_parser_internal, token, identifier,
    module_declaration_parser_internal, import_declaration_parser_internal,
    function_declaration_parser_internal, field_declaration_parser_internal,
    initializer_declaration_parser_internal,
    emit_struct_declaration,
    StructDeclarationData, StructBodyItem,
};
use crate::type_alias::type_alias_declaration_parser_internal;
use crate::type_param::{type_parameter_list_parser, where_clause_parser, conformance_list_parser};
use crate::common::ConformanceListData;

/// Represents a struct declaration: (visibility)? struct Name[T]? (where ...)? { ... }
///
/// The declaration is stored as a lossless syntax tree. All data is derived
/// from the tree rather than stored separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructDeclaration {
    pub syntax: SyntaxNode,
    pub span: Span,
}

impl StructDeclaration {
    /// Create a new StructDeclaration from events and source text
    pub fn from_events(source: &str, events: Vec<crate::event::Event>, span: Span) -> Self {
        let builder = TreeBuilder::new(source, events);
        let syntax = builder.build();
        Self { syntax, span }
    }

    /// Get the struct name from this declaration
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
        let visibility_node = self.syntax
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

    /// Get child declaration items (nested structs, imports, modules, fields, functions, initializers)
    pub fn children(&self) -> Vec<SyntaxNode> {
        self.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::StructBody)
            .map(|body| {
                body.children()
                    .filter(|child| {
                        matches!(
                            child.kind(),
                            SyntaxKind::StructDeclaration
                                | SyntaxKind::ImportDeclaration
                                | SyntaxKind::ModuleDeclaration
                                | SyntaxKind::FieldDeclaration
                                | SyntaxKind::FunctionDeclaration
                                | SyntaxKind::InitializerDeclaration
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Internal parser for struct body items
///
/// Struct bodies can contain: fields, functions, initializers, nested structs, type aliases, modules, and imports.
fn struct_body_item_parser_internal(
    struct_parser: impl Parser<Token, StructDeclarationData, Error = Simple<Token>> + Clone
) -> impl Parser<Token, StructBodyItem, Error = Simple<Token>> + Clone {
    let module_parser = module_declaration_parser_internal()
        .map(|(module_span, path)| StructBodyItem::Module(module_span, path));

    let import_parser = import_declaration_parser_internal()
        .map(|(import_span, path, alias, items)| StructBodyItem::Import(import_span, path, alias, items));

    let nested_struct_parser = struct_parser.map(StructBodyItem::Struct);

    let initializer_parser = initializer_declaration_parser_internal().map(StructBodyItem::Initializer);

    let function_parser = function_declaration_parser_internal().map(StructBodyItem::Function);

    let type_alias_parser = type_alias_declaration_parser_internal().map(StructBodyItem::TypeAlias);

    let field_parser = field_declaration_parser_internal().map(StructBodyItem::Field);

    module_parser
        .or(import_parser)
        .or(nested_struct_parser)
        .or(initializer_parser)
        .or(type_alias_parser) // Check type alias before function (both can have visibility)
        .or(function_parser)
        .or(field_parser)
}

/// Internal Chumsky parser for struct declaration
///
/// This is the single source of truth for struct declaration parsing.
pub fn struct_declaration_parser_internal() -> impl Parser<Token, StructDeclarationData, Error = Simple<Token>> + Clone {
    recursive(|struct_parser| {
        visibility_parser_internal()
            .then(token(Token::Struct))
            .then(identifier())
            .then(type_parameter_list_parser().or_not())
            .then(conformance_list_parser().or_not())
            .then(where_clause_parser().or_not())
            .then(token(Token::LBrace))
            .then(struct_body_item_parser_internal(struct_parser).repeated())
            .then(token(Token::RBrace))
            .map(|((((((((visibility, struct_span), name_span), type_params), conformances), where_clause), lbrace_span), body), rbrace_span)| {
                StructDeclarationData {
                    visibility,
                    struct_span,
                    name_span,
                    type_params,
                    conformances: conformances.map(|(colon_span, types)| ConformanceListData {
                        colon_span,
                        conformances: types,
                    }),
                    where_clause,
                    lbrace_span,
                    body,
                    rbrace_span,
                }
            })
    })
}

/// Parse a struct declaration and emit events
///
/// This is the primary event-driven parser function for struct declarations.
pub fn parse_struct_declaration<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let end_pos = source.len();
    let stream = chumsky::Stream::from_iter(end_pos..end_pos, tokens);

    match struct_declaration_parser_internal().parse(stream) {
        Ok(data) => {
            emit_struct_declaration(sink, data);
        }
        Err(errors) => {
            for error in errors {
                let span = error.span();
                sink.error_at(format!("Parse error: {:?}", error), span);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_lexer::lex;

    /// Helper to parse source code and return a StructDeclaration
    fn parse(source: &str) -> StructDeclaration {
        let tokens: Vec<_> = lex(source)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();
        let mut sink = EventSink::new();
        parse_struct_declaration(source, tokens.into_iter(), &mut sink);
        let tree = TreeBuilder::new(source, sink.into_events()).build();
        StructDeclaration { syntax: tree, span: 0..source.len() }
    }

    /// Helper to check if a syntax node exists as a child
    fn has_child(decl: &StructDeclaration, kind: SyntaxKind) -> bool {
        decl.syntax.children().any(|child| child.kind() == kind)
    }

    #[test]
    fn test_struct_declaration_basic() {
        let decl = parse("struct Foo { }");
        assert_eq!(decl.name(), Some("Foo".to_string()));
        assert_eq!(decl.visibility(), None);
        assert_eq!(decl.syntax.kind(), SyntaxKind::StructDeclaration);
    }

    #[test]
    fn test_struct_declaration_with_visibility() {
        let decl = parse("public struct Bar { }");
        assert_eq!(decl.name(), Some("Bar".to_string()));
        assert_eq!(decl.visibility(), Some(SyntaxKind::Public));
    }

    #[test]
    fn test_struct_declaration_with_nested_struct() {
        let decl = parse("struct Outer { struct Inner { } }");
        assert_eq!(decl.name(), Some("Outer".to_string()));
        let children = decl.children();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].kind(), SyntaxKind::StructDeclaration);
    }

    #[test]
    fn test_struct_declaration_with_type_params() {
        let decl = parse("struct Box[T] { }");
        assert_eq!(decl.name(), Some("Box".to_string()));
        assert!(has_child(&decl, SyntaxKind::TypeParameterList));
    }

    #[test]
    fn test_struct_declaration_with_where_clause() {
        let decl = parse("struct Set[T] where T: Equatable { }");
        assert_eq!(decl.name(), Some("Set".to_string()));
        assert!(has_child(&decl, SyntaxKind::WhereClause));
    }

    #[test]
    fn test_struct_with_field() {
        let decl = parse("struct Point { let x: Int }");
        let children = decl.children();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].kind(), SyntaxKind::FieldDeclaration);
    }

    #[test]
    fn test_struct_with_function() {
        let decl = parse("struct Calculator { func add(a: Int, b: Int) -> Int { } }");
        let children = decl.children();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].kind(), SyntaxKind::FunctionDeclaration);
    }

    #[test]
    fn test_struct_with_multiple_members() {
        let decl = parse("struct Person { let name: String var age: Int func greet() { } }");
        let children = decl.children();
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].kind(), SyntaxKind::FieldDeclaration);
        assert_eq!(children[1].kind(), SyntaxKind::FieldDeclaration);
        assert_eq!(children[2].kind(), SyntaxKind::FunctionDeclaration);
    }

    #[test]
    fn test_struct_with_conformance() {
        let decl = parse("struct Point: Drawable { }");
        assert_eq!(decl.name(), Some("Point".to_string()));
        assert!(has_child(&decl, SyntaxKind::ConformanceList));
    }

    #[test]
    fn test_struct_with_multiple_conformances() {
        let decl = parse("struct Point: Drawable, Equatable { }");
        let conformance_list = decl.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::ConformanceList)
            .expect("Expected ConformanceList node");
        let conformance_count = conformance_list
            .children()
            .filter(|c| c.kind() == SyntaxKind::ConformanceItem)
            .count();
        assert_eq!(conformance_count, 2);
    }

    #[test]
    fn test_struct_with_generic_conformance() {
        let decl = parse("struct IntBox: Container[Int] { }");
        assert_eq!(decl.name(), Some("IntBox".to_string()));
        assert!(has_child(&decl, SyntaxKind::ConformanceList));
    }

    #[test]
    fn test_struct_full_syntax() {
        let decl = parse("struct Box[T]: Container[T] where T: Equatable { }");
        assert_eq!(decl.name(), Some("Box".to_string()));
        assert!(has_child(&decl, SyntaxKind::TypeParameterList));
        assert!(has_child(&decl, SyntaxKind::ConformanceList));
        assert!(has_child(&decl, SyntaxKind::WhereClause));
    }

    #[test]
    fn test_struct_with_field_and_initializer() {
        // This tests the bug: struct with 2+ fields and an initializer fails to parse
        let decl = parse("struct Point { var x: Int var y: Int init() { } }");
        let children = decl.children();
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].kind(), SyntaxKind::FieldDeclaration);
        assert_eq!(children[1].kind(), SyntaxKind::FieldDeclaration);
        assert_eq!(children[2].kind(), SyntaxKind::InitializerDeclaration);
    }

    #[test]
    fn test_struct_with_single_field_and_initializer() {
        let decl = parse("struct Wrapper { var value: Int init() { } }");
        let children = decl.children();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].kind(), SyntaxKind::FieldDeclaration);
        assert_eq!(children[1].kind(), SyntaxKind::InitializerDeclaration);
    }

    #[test]
    fn test_struct_with_semicolon_separated_fields() {
        // Test inline fields separated by semicolons
        let decl = parse("struct Point { var x: Int; var y: Int }");
        let children = decl.children();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].kind(), SyntaxKind::FieldDeclaration);
        assert_eq!(children[1].kind(), SyntaxKind::FieldDeclaration);
    }

    #[test]
    fn test_struct_with_semicolon_separated_fields_trailing() {
        // Test inline fields with trailing semicolon
        let decl = parse("struct Point { var x: Int; var y: Int; }");
        let children = decl.children();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].kind(), SyntaxKind::FieldDeclaration);
        assert_eq!(children[1].kind(), SyntaxKind::FieldDeclaration);
    }

    #[test]
    fn test_struct_with_mixed_field_separators() {
        // Test mixing semicolons and newlines
        let decl = parse("struct Point { var x: Int; var y: Int\nvar z: Int }");
        let children = decl.children();
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].kind(), SyntaxKind::FieldDeclaration);
        assert_eq!(children[1].kind(), SyntaxKind::FieldDeclaration);
        assert_eq!(children[2].kind(), SyntaxKind::FieldDeclaration);
    }
}
