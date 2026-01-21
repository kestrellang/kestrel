//! Function declaration parsing
//!
//! This module is the single source of truth for function declaration parsing.
//! Functions support generics with type parameters and where clauses.

use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::common::{emit_function_declaration, function_declaration_parser_internal};
use crate::event::{EventSink, TreeBuilder};
use crate::input::{create_input, prepare_tokens};

/// Represents a function declaration: (visibility)? (static)? fn name[T]?(params) (-> return_type)? (where ...)? { }
///
/// The declaration is stored as a lossless syntax tree. All data is derived
/// from the tree rather than stored separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDeclaration {
    pub syntax: SyntaxNode,
    pub span: Span,
}

impl FunctionDeclaration {
    /// Create a new FunctionDeclaration from events and source text
    pub fn from_events(source: &str, events: Vec<crate::event::Event>, span: Span) -> Self {
        let builder = TreeBuilder::new(source, events);
        let syntax = builder.build();
        Self { syntax, span }
    }

    /// Get the function name from this declaration
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

    /// Check if this function has the static modifier
    pub fn is_static(&self) -> bool {
        self.syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::StaticModifier)
    }

    /// Get the parameter list node
    pub fn parameter_list(&self) -> Option<SyntaxNode> {
        self.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::ParameterList)
    }

    /// Get the return type node if present
    pub fn return_type(&self) -> Option<SyntaxNode> {
        self.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::ReturnType)
    }

    /// Get the function body node
    pub fn body(&self) -> Option<SyntaxNode> {
        self.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::FunctionBody)
    }

    /// Check if this function has type parameters
    pub fn has_type_parameters(&self) -> bool {
        self.syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::TypeParameterList)
    }

    /// Check if this function has a where clause
    pub fn has_where_clause(&self) -> bool {
        self.syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::WhereClause)
    }
}

/// Parse a function declaration and emit events
///
/// This is the primary event-driven parser function for function declarations.
pub fn parse_function_declaration<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    use chumsky::Parser;

    let prepared = prepare_tokens(tokens);
    let input = create_input(&prepared, source.len());

    match function_declaration_parser_internal()
        .parse(input)
        .into_result()
    {
        Ok(data) => {
            emit_function_declaration(sink, data);
        },
        Err(errors) => {
            for error in errors {
                let span = error.span();
                sink.error_at(format!("Parse error: {:?}", error), *span);
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_lexer::lex;

    #[test]
    fn test_function_declaration_basic() {
        let source = "func test() { }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_function_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FunctionDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("test".to_string()));
        assert_eq!(decl.visibility(), None);
        assert!(!decl.is_static());
    }

    #[test]
    fn test_function_declaration_with_visibility() {
        let source = "public func greet() { }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_function_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FunctionDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("greet".to_string()));
        assert_eq!(decl.visibility(), Some(SyntaxKind::Public));
    }

    #[test]
    fn test_function_declaration_static() {
        let source = "static func create() { }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_function_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FunctionDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("create".to_string()));
        assert!(decl.is_static());
    }

    #[test]
    fn test_function_declaration_with_params() {
        let source = "func add(a: Int, b: Int) { }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_function_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FunctionDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("add".to_string()));
        assert!(decl.parameter_list().is_some());
    }

    #[test]
    fn test_function_declaration_with_labeled_param() {
        let source = "func greet(with name: String) { }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_function_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FunctionDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("greet".to_string()));
        assert!(decl.parameter_list().is_some());
    }

    #[test]
    fn test_function_declaration_with_return_type() {
        let source = "func multiply(x: Int, y: Int) -> Int { }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_function_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FunctionDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("multiply".to_string()));
        assert!(decl.return_type().is_some());
    }

    #[test]
    fn test_function_declaration_with_generics() {
        let source = "func identity[T](value: T) -> T { }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_function_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FunctionDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("identity".to_string()));
        assert!(decl.has_type_parameters());
    }

    #[test]
    fn test_function_declaration_with_where_clause() {
        let source = "func compare[T](a: T, b: T) -> Bool where T: Equatable { }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_function_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FunctionDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("compare".to_string()));
        assert!(decl.has_type_parameters());
        assert!(decl.has_where_clause());
    }

    #[test]
    fn test_function_declaration_full() {
        let source = "public static func calculate(value: Float, multiplier: Float) -> Float { }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_function_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FunctionDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("calculate".to_string()));
        assert_eq!(decl.visibility(), Some(SyntaxKind::Public));
        assert!(decl.is_static());
        assert!(decl.parameter_list().is_some());
        assert!(decl.return_type().is_some());
    }

    #[test]
    fn test_function_with_deinit_statement() {
        let source = "func example() { let x: Int = 0; deinit x; }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_function_declaration(source, tokens.into_iter(), &mut sink);

        let events = sink.into_events();

        // Check for parse errors
        let errors: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                crate::event::Event::Error { message, .. } => Some(message.clone()),
                _ => None,
            })
            .collect();

        assert!(errors.is_empty(), "Got parse errors: {:?}", errors);

        let tree = TreeBuilder::new(source, events).build();
        let decl = FunctionDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("example".to_string()));
        assert!(decl.body().is_some(), "Function should have a body");
    }
}
