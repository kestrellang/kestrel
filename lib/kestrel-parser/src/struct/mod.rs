//! Struct declaration parsing
//!
//! This module is the single source of truth for struct declaration parsing.
//! Struct bodies can contain: fields, functions, nested structs, modules, and imports.
//!
//! Note: The actual parsing is delegated to the unified type_decl module to handle
//! mutual recursion between structs and enums efficiently.

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::common::{
    AttributeData, ConformanceListData, TypeDeclarationBodyItem, emit_attribute_list, emit_name,
    emit_type_declaration_body_item, emit_visibility,
};
use crate::event::{EventSink, TreeBuilder};
use crate::input::{ParserExtra, ParserInput};
use crate::parse_and_emit;
use crate::type_decl::struct_declaration_parser_unified;
use crate::type_param::{
    TypeParameterData, WhereClauseData, emit_conformance_list, emit_type_parameter_list,
    emit_where_clause,
};

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

    /// Get child declaration items (nested structs, nested enums, imports, modules, fields, functions, initializers, deinit)
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
                                | SyntaxKind::EnumDeclaration
                                | SyntaxKind::ImportDeclaration
                                | SyntaxKind::ModuleDeclaration
                                | SyntaxKind::FieldDeclaration
                                | SyntaxKind::FunctionDeclaration
                                | SyntaxKind::InitializerDeclaration
                                | SyntaxKind::DeinitDeclaration
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Internal Chumsky parser for struct declaration
///
/// Raw parsed data for struct declaration internals
#[derive(Debug, Clone)]
pub struct StructDeclarationData {
    pub attributes: Vec<AttributeData>,
    pub visibility: Option<(Token, Span)>,
    pub struct_span: Span,
    pub name_span: Span,
    pub type_params: Option<(Span, Vec<TypeParameterData>, Span)>,
    pub conformances: Option<ConformanceListData>,
    pub where_clause: Option<WhereClauseData>,
    pub lbrace_span: Span,
    pub body: Vec<TypeDeclarationBodyItem>,
    pub rbrace_span: Span,
}

/// This delegates to the unified type_decl parser which handles both struct and enum
/// in a single recursive context to avoid stack overflow on deeply nested types.
pub fn struct_declaration_parser_internal<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, StructDeclarationData, ParserExtra<'tokens>> + Clone {
    struct_declaration_parser_unified()
}

/// Emit events for a struct declaration.
///
/// Destructures `StructDeclarationData` without a `..` rest pattern: adding a
/// field forces this function to stop compiling until the new field is
/// handled in emission (see the `EmitSyntax` trait docs).
pub fn emit_struct_declaration(sink: &mut EventSink, data: StructDeclarationData) {
    let StructDeclarationData {
        attributes,
        visibility,
        struct_span,
        name_span,
        type_params,
        conformances,
        where_clause,
        lbrace_span,
        body,
        rbrace_span,
    } = data;

    sink.start_node(SyntaxKind::StructDeclaration);

    emit_attribute_list(sink, &attributes);
    emit_visibility(sink, visibility);
    sink.add_token(SyntaxKind::Struct, struct_span);
    emit_name(sink, name_span);

    if let Some((lbracket, params, rbracket)) = type_params {
        emit_type_parameter_list(sink, lbracket, params, rbracket);
    }

    if let Some(conf) = conformances {
        emit_conformance_list(sink, conf.colon_span, &conf.conformances);
    }

    if let Some(wc) = where_clause {
        emit_where_clause(sink, wc);
    }

    sink.start_node(SyntaxKind::StructBody);
    sink.add_token(SyntaxKind::LBrace, lbrace_span);

    for item in body {
        emit_type_declaration_body_item(sink, item);
    }

    sink.add_token(SyntaxKind::RBrace, rbrace_span);
    sink.finish_node(); // StructBody

    sink.finish_node(); // StructDeclaration
}

impl crate::event::EmitSyntax for StructDeclarationData {
    fn emit(self, sink: &mut EventSink) {
        emit_struct_declaration(sink, self);
    }
}

/// Parse a struct declaration and emit events
///
/// This is the primary event-driven parser function for struct declarations.
pub fn parse_struct_declaration<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    parse_and_emit!(
        source,
        tokens,
        sink,
        struct_declaration_parser_internal(),
        emit_struct_declaration
    );
}

#[cfg(test)]
mod emit_syntax_trait_tests {
    use super::*;
    use crate::event::{EmitSyntax, EventSink, TreeBuilder};
    use kestrel_lexer::lex;

    /// Calling `.emit(sink)` on a parsed `StructDeclarationData` must produce
    /// the same tree as calling `emit_struct_declaration(sink, data)`. This
    /// smoke-tests the EmitSyntax trait and locks in its contract.
    #[test]
    fn emit_syntax_impl_matches_free_function() {
        let source = "struct Foo { }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|s| (s.value, s.span))
            .collect();

        let mut sink_fn = EventSink::new(0);
        parse_struct_declaration(source, tokens.clone().into_iter(), &mut sink_fn);
        let tree_fn = TreeBuilder::new(source, sink_fn.into_events()).build();

        // Build the same tree by calling `.emit(sink)` through the trait.
        use chumsky::Parser;
        use crate::input::{create_input, prepare_tokens};
        let prepared = prepare_tokens(tokens.into_iter());
        let input = create_input(&prepared, source.len());
        let data = struct_declaration_parser_internal()
            .parse(input)
            .into_output()
            .expect("struct should parse");
        let mut sink_trait = EventSink::new(0);
        data.emit(&mut sink_trait);
        let tree_trait = TreeBuilder::new(source, sink_trait.into_events()).build();

        assert_eq!(tree_fn.text().to_string(), tree_trait.text().to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_lexer::lex;

    /// Helper to parse source code and return a StructDeclaration
    fn parse(source: &str) -> StructDeclaration {
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();
        let mut sink = EventSink::new(0);
        parse_struct_declaration(source, tokens.into_iter(), &mut sink);
        let tree = TreeBuilder::new(source, sink.into_events()).build();
        StructDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        }
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
    fn test_struct_with_attribute() {
        let decl = parse("@dummy struct Point { }");
        assert_eq!(decl.name(), Some("Point".to_string()));
        // Check for AttributeList as a child
        assert!(
            has_child(&decl, SyntaxKind::AttributeList),
            "Expected AttributeList as child of StructDeclaration"
        );

        // Verify the attribute structure in more detail
        let attr_list = decl
            .syntax
            .children()
            .find(|c| c.kind() == SyntaxKind::AttributeList)
            .expect("AttributeList should exist");

        let attr = attr_list
            .children()
            .find(|c| c.kind() == SyntaxKind::Attribute)
            .expect("Attribute should exist in AttributeList");

        // Check that we have the @ token and identifier
        let has_at = attr
            .children_with_tokens()
            .filter_map(|c| c.into_token())
            .any(|t| t.kind() == SyntaxKind::At);
        assert!(has_at, "Attribute should have @ token");

        let has_name = attr
            .children_with_tokens()
            .filter_map(|c| c.into_token())
            .any(|t| t.kind() == SyntaxKind::Identifier && t.text() == "dummy");
        assert!(has_name, "Attribute should have 'dummy' identifier");
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
    fn test_struct_with_function_body() {
        // Test function with actual body content (to compare with deinit_with_body)
        let decl = parse("struct Calculator { func add() { let x = 1; } }");
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
        let conformance_list = decl
            .syntax
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

    #[test]
    fn test_struct_with_deinit() {
        let decl = parse("struct FileHandle { var fd: Int deinit { } }");
        let children = decl.children();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].kind(), SyntaxKind::FieldDeclaration);
        assert_eq!(children[1].kind(), SyntaxKind::DeinitDeclaration);
    }

    #[test]
    fn test_struct_with_init_and_deinit() {
        let decl = parse("struct Resource { var handle: Int init() { } deinit { } }");
        let children = decl.children();
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].kind(), SyntaxKind::FieldDeclaration);
        assert_eq!(children[1].kind(), SyntaxKind::InitializerDeclaration);
        assert_eq!(children[2].kind(), SyntaxKind::DeinitDeclaration);
    }

    #[test]
    fn test_struct_deinit_with_body() {
        // Test deinit with actual body content
        let decl = parse("struct Connection { var socket: Int deinit { let x = 1; } }");
        let children = decl.children();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].kind(), SyntaxKind::FieldDeclaration);
        assert_eq!(children[1].kind(), SyntaxKind::DeinitDeclaration);

        // Verify the deinit has a function body
        let deinit_node = &children[1];
        let has_body = deinit_node
            .children()
            .any(|c| c.kind() == SyntaxKind::FunctionBody);
        assert!(has_body, "deinit should have a FunctionBody child");
    }

    #[test]
    fn test_struct_with_trailing_comma_in_conformance() {
        // Test that trailing comma before opening brace is allowed
        let decl = parse("struct Point: Drawable, Equatable, { }");
        assert_eq!(decl.name(), Some("Point".to_string()));
        let conformance_list = decl
            .syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::ConformanceList)
            .expect("Expected ConformanceList node");
        let conformance_count = conformance_list
            .children()
            .filter(|c| c.kind() == SyntaxKind::ConformanceItem)
            .count();
        assert_eq!(conformance_count, 2);
    }
}
