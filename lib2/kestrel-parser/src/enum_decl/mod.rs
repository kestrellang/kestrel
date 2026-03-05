//! Enum declaration parsing
//!
//! This module is the single source of truth for enum declaration parsing.
//! Enum bodies can contain: cases, functions, initializers, nested structs/enums,
//! type aliases, modules, and imports.
//!
//! Note: The actual parsing is delegated to the unified type_decl module to handle
//! mutual recursion between structs and enums efficiently.

use kestrel_lexer2::Token;
use kestrel_span2::Span;
use kestrel_syntax_tree2::{SyntaxKind, SyntaxNode};

use crate::common::{EnumDeclarationData, emit_enum_declaration};
use crate::event::{EventSink, TreeBuilder};
use crate::input::{ParserExtra, ParserInput, create_input, prepare_tokens};
use crate::type_decl::enum_declaration_parser_unified;

use chumsky::prelude::*;

/// Represents an enum declaration: (visibility)? (indirect)? enum Name[T]? (: Conformances)? (where ...)? { ... }
///
/// The declaration is stored as a lossless syntax tree. All data is derived
/// from the tree rather than stored separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDeclaration {
    pub syntax: SyntaxNode,
    pub span: Span,
}

impl EnumDeclaration {
    /// Create a new EnumDeclaration from events and source text
    pub fn from_events(source: &str, events: Vec<crate::event::Event>, span: Span) -> Self {
        let builder = TreeBuilder::new(source, events);
        let syntax = builder.build();
        Self { syntax, span }
    }

    /// Get the enum name from this declaration
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

    /// Check if this enum has the `indirect` modifier
    pub fn is_indirect(&self) -> bool {
        self.syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::IndirectModifier)
    }

    /// Get child declaration items (cases, nested structs, imports, modules, functions, initializers)
    pub fn children(&self) -> Vec<SyntaxNode> {
        self.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::EnumBody)
            .map(|body| {
                body.children()
                    .filter(|child| {
                        matches!(
                            child.kind(),
                            SyntaxKind::EnumCaseDeclaration
                                | SyntaxKind::StructDeclaration
                                | SyntaxKind::EnumDeclaration
                                | SyntaxKind::ImportDeclaration
                                | SyntaxKind::ModuleDeclaration
                                | SyntaxKind::FieldDeclaration
                                | SyntaxKind::FunctionDeclaration
                                | SyntaxKind::InitializerDeclaration
                                | SyntaxKind::TypeAliasDeclaration
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all enum cases in this declaration
    pub fn cases(&self) -> Vec<SyntaxNode> {
        self.children()
            .into_iter()
            .filter(|child| child.kind() == SyntaxKind::EnumCaseDeclaration)
            .collect()
    }
}

/// Internal Chumsky parser for enum declaration
///
/// This delegates to the unified type_decl parser which handles both struct and enum
/// in a single recursive context to avoid stack overflow on deeply nested types.
pub fn enum_declaration_parser_internal<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, EnumDeclarationData, ParserExtra<'tokens>> + Clone {
    enum_declaration_parser_unified()
}

/// Parse an enum declaration and emit events
///
/// This is the primary event-driven parser function for enum declarations.
pub fn parse_enum_declaration<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let prepared = prepare_tokens(tokens);
    let input = create_input(&prepared, source.len());

    match enum_declaration_parser_internal()
        .parse(input)
        .into_result()
    {
        Ok(data) => {
            emit_enum_declaration(sink, data);
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

    /// Helper to parse source code and return an EnumDeclaration
    fn parse(source: &str) -> EnumDeclaration {
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();
        let mut sink = EventSink::new(0);
        parse_enum_declaration(source, tokens.into_iter(), &mut sink);
        let tree = TreeBuilder::new(source, sink.into_events()).build();
        EnumDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        }
    }

    /// Helper to check if a syntax node exists as a child
    fn has_child(decl: &EnumDeclaration, kind: SyntaxKind) -> bool {
        decl.syntax.children().any(|child| child.kind() == kind)
    }

    #[test]
    fn test_enum_declaration_basic() {
        let decl = parse("enum Color { }");
        assert_eq!(decl.name(), Some("Color".to_string()));
        assert_eq!(decl.visibility(), None);
        assert!(!decl.is_indirect());
        assert_eq!(decl.syntax.kind(), SyntaxKind::EnumDeclaration);
    }

    #[test]
    fn test_enum_declaration_with_visibility() {
        let decl = parse("public enum Direction { }");
        assert_eq!(decl.name(), Some("Direction".to_string()));
        assert_eq!(decl.visibility(), Some(SyntaxKind::Public));
    }

    #[test]
    fn test_enum_declaration_with_indirect() {
        let decl = parse("indirect enum Tree { }");
        assert_eq!(decl.name(), Some("Tree".to_string()));
        assert!(decl.is_indirect());
    }

    #[test]
    fn test_enum_declaration_with_visibility_and_indirect() {
        let decl = parse("public indirect enum LinkedList { }");
        assert_eq!(decl.name(), Some("LinkedList".to_string()));
        assert_eq!(decl.visibility(), Some(SyntaxKind::Public));
        assert!(decl.is_indirect());
    }

    #[test]
    fn test_enum_with_simple_case() {
        let decl = parse("enum Color { case Red }");
        let cases = decl.cases();
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].kind(), SyntaxKind::EnumCaseDeclaration);
    }

    #[test]
    fn test_enum_with_multiple_cases() {
        let decl = parse("enum Color { case Red case Green case Blue }");
        let cases = decl.cases();
        assert_eq!(cases.len(), 3);
    }

    #[test]
    fn test_enum_case_with_associated_values() {
        let decl = parse("enum Result { case Success(value: Int) }");
        let cases = decl.cases();
        assert_eq!(cases.len(), 1);

        // Check that the case has parameters
        let case_node = &cases[0];
        let has_param_list = case_node
            .children()
            .any(|c| c.kind() == SyntaxKind::EnumCaseParameterList);
        assert!(has_param_list);
    }

    #[test]
    fn test_enum_case_with_unnamed_parameter() {
        // Unnamed parameter: just Type without label
        let decl = parse("enum Option[T] { case Some(T) case None }");
        let cases = decl.cases();
        assert_eq!(cases.len(), 2);

        // Check that Some has parameters
        let some_case = &cases[0];
        let has_param_list = some_case
            .children()
            .any(|c| c.kind() == SyntaxKind::EnumCaseParameterList);
        assert!(has_param_list);

        // Check that None has no parameters
        let none_case = &cases[1];
        let none_has_params = none_case
            .children()
            .any(|c| c.kind() == SyntaxKind::EnumCaseParameterList);
        assert!(!none_has_params);
    }

    #[test]
    fn test_enum_case_with_multiple_unnamed_parameters() {
        let decl = parse("enum Pair[A, B] { case Both(A, B) }");
        let cases = decl.cases();
        assert_eq!(cases.len(), 1);

        let case_node = &cases[0];
        let param_list = case_node
            .children()
            .find(|c| c.kind() == SyntaxKind::EnumCaseParameterList);
        assert!(param_list.is_some());

        let param_count = param_list
            .unwrap()
            .children()
            .filter(|c| c.kind() == SyntaxKind::EnumCaseParameter)
            .count();
        assert_eq!(param_count, 2);
    }

    #[test]
    fn test_enum_case_with_multiple_associated_values() {
        let decl = parse("enum Event { case Click(x: Int, y: Int) }");
        let cases = decl.cases();
        assert_eq!(cases.len(), 1);

        // Count parameters in the case
        let case_node = &cases[0];
        let param_list = case_node
            .children()
            .find(|c| c.kind() == SyntaxKind::EnumCaseParameterList);
        assert!(param_list.is_some());

        let param_count = param_list
            .unwrap()
            .children()
            .filter(|c| c.kind() == SyntaxKind::EnumCaseParameter)
            .count();
        assert_eq!(param_count, 2);
    }

    #[test]
    fn test_enum_with_type_params() {
        let decl = parse("enum Option[T] { }");
        assert_eq!(decl.name(), Some("Option".to_string()));
        assert!(has_child(&decl, SyntaxKind::TypeParameterList));
    }

    #[test]
    fn test_enum_with_conformance() {
        let decl = parse("enum Status: Equatable { }");
        assert_eq!(decl.name(), Some("Status".to_string()));
        assert!(has_child(&decl, SyntaxKind::ConformanceList));
    }

    #[test]
    fn test_enum_with_where_clause() {
        let decl = parse("enum Container[T] where T: Equatable { }");
        assert_eq!(decl.name(), Some("Container".to_string()));
        assert!(has_child(&decl, SyntaxKind::WhereClause));
    }

    #[test]
    fn test_enum_full_syntax() {
        let decl = parse(
            "public indirect enum Result[T, E]: Equatable where E: Error { case Success(value: T) case Failure(error: E) }",
        );
        assert_eq!(decl.name(), Some("Result".to_string()));
        assert_eq!(decl.visibility(), Some(SyntaxKind::Public));
        assert!(decl.is_indirect());
        assert!(has_child(&decl, SyntaxKind::TypeParameterList));
        assert!(has_child(&decl, SyntaxKind::ConformanceList));
        assert!(has_child(&decl, SyntaxKind::WhereClause));
        assert_eq!(decl.cases().len(), 2);
    }

    #[test]
    fn test_enum_with_function() {
        let decl = parse("enum Color { case Red func describe() -> String { } }");
        let children = decl.children();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].kind(), SyntaxKind::EnumCaseDeclaration);
        assert_eq!(children[1].kind(), SyntaxKind::FunctionDeclaration);
    }

    #[test]
    fn test_enum_with_initializer() {
        let decl = parse("enum Direction { case North init() { } }");
        let children = decl.children();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].kind(), SyntaxKind::EnumCaseDeclaration);
        assert_eq!(children[1].kind(), SyntaxKind::InitializerDeclaration);
    }

    #[test]
    fn test_enum_with_computed_property() {
        // Test multiline computed property in enum
        let decl = parse(
            "enum Optional[T] {
                case Some(T)
                case None
                public var isSome: Bool {
                    get {
                        true
                    }
                }
            }",
        );
        let children = decl.children();
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].kind(), SyntaxKind::EnumCaseDeclaration);
        assert_eq!(children[1].kind(), SyntaxKind::EnumCaseDeclaration);
        assert_eq!(children[2].kind(), SyntaxKind::FieldDeclaration);
    }

    #[test]
    fn test_enum_with_shorthand_computed_property() {
        // Test shorthand computed property (just a block, no get/set)
        let decl = parse(
            "enum Optional[T] {
                case Some(T)
                case None
                public var isSome: Bool {
                    match self {
                        .Some(_) => true,
                        .None => false
                    }
                }
            }",
        );
        let children = decl.children();
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].kind(), SyntaxKind::EnumCaseDeclaration);
        assert_eq!(children[1].kind(), SyntaxKind::EnumCaseDeclaration);
        assert_eq!(children[2].kind(), SyntaxKind::FieldDeclaration);
    }
}
