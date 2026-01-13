//! Subscript declaration parsing
//!
//! This module is the single source of truth for subscript declaration parsing.
//! Subscripts support generics with type parameters and where clauses.

use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::common::{emit_subscript_declaration, subscript_declaration_parser_internal};
use crate::event::{EventSink, TreeBuilder};
use crate::input::{create_input, prepare_tokens};

/// Represents a subscript declaration: (visibility)? (static)? subscript[T]?(params) -> Type (where ...)? { }
///
/// The declaration is stored as a lossless syntax tree. All data is derived
/// from the tree rather than stored separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscriptDeclaration {
    pub syntax: SyntaxNode,
    pub span: Span,
}

impl SubscriptDeclaration {
    /// Create a new SubscriptDeclaration from events and source text
    pub fn from_events(source: &str, events: Vec<crate::event::Event>, span: Span) -> Self {
        let builder = TreeBuilder::new(source, events);
        let syntax = builder.build();
        Self { syntax, span }
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

    /// Check if this subscript has the static modifier
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

    /// Get the return type node
    pub fn return_type(&self) -> Option<SyntaxNode> {
        self.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::ReturnType)
    }

    /// Get the subscript body node
    pub fn body(&self) -> Option<SyntaxNode> {
        self.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::SubscriptBody)
    }

    /// Get the property accessors node if present (for explicit get/set)
    pub fn property_accessors(&self) -> Option<SyntaxNode> {
        self.body()?
            .children()
            .find(|child| child.kind() == SyntaxKind::PropertyAccessors)
    }

    /// Get the getter clause if present (for explicit getter syntax)
    pub fn getter_clause(&self) -> Option<SyntaxNode> {
        // First try in PropertyAccessors (explicit form)
        if let Some(accessors) = self.property_accessors() {
            if let Some(getter) = accessors.children().find(|child| child.kind() == SyntaxKind::GetterClause) {
                return Some(getter);
            }
        }
        None
    }

    /// Get the setter clause if present
    pub fn setter_clause(&self) -> Option<SyntaxNode> {
        // Try in PropertyAccessors
        if let Some(accessors) = self.property_accessors() {
            if let Some(setter) = accessors.children().find(|child| child.kind() == SyntaxKind::SetterClause) {
                return Some(setter);
            }
        }
        None
    }

    /// Get the getter body (for shorthand or explicit form)
    pub fn getter_body(&self) -> Option<SyntaxNode> {
        let body = self.body()?;

        // For shorthand form, the body is directly a CodeBlock
        if let Some(code_block) = body.children().find(|child| child.kind() == SyntaxKind::CodeBlock) {
            // Check if it's inside a PropertyAccessors (explicit form)
            if self.property_accessors().is_none() {
                return Some(code_block);
            }
        }

        // For explicit form, find the code block in GetterClause
        self.getter_clause()?
            .children()
            .find(|child| child.kind() == SyntaxKind::CodeBlock)
    }

    /// Get the setter body if present
    pub fn setter_body(&self) -> Option<SyntaxNode> {
        self.setter_clause()?
            .children()
            .find(|child| child.kind() == SyntaxKind::CodeBlock)
    }

    /// Check if this subscript is getter-only (no setter)
    pub fn is_getter_only(&self) -> bool {
        self.setter_clause().is_none() && !self.has_protocol_setter_requirement()
    }

    /// Check if this is a protocol requirement (has `{ get }` or `{ get set }` without bodies)
    pub fn is_protocol_requirement(&self) -> bool {
        if let Some(accessors) = self.property_accessors() {
            // Protocol requirements have Get/Set tokens but no GetterClause/SetterClause children
            let has_getter_clause = accessors.children().any(|c| c.kind() == SyntaxKind::GetterClause);
            let has_get_token = accessors.children_with_tokens()
                .filter_map(|e| e.into_token())
                .any(|t| t.kind() == SyntaxKind::Get);

            // If there's a Get token but no GetterClause, it's a protocol requirement
            has_get_token && !has_getter_clause
        } else {
            false
        }
    }

    /// Check if this has a protocol setter requirement (has `{ get set }` without bodies)
    fn has_protocol_setter_requirement(&self) -> bool {
        if let Some(accessors) = self.property_accessors() {
            let has_setter_clause = accessors.children().any(|c| c.kind() == SyntaxKind::SetterClause);
            let has_set_token = accessors.children_with_tokens()
                .filter_map(|e| e.into_token())
                .any(|t| t.kind() == SyntaxKind::Set);

            // If there's a Set token but no SetterClause, it's a protocol setter requirement
            has_set_token && !has_setter_clause
        } else {
            false
        }
    }

    /// Check if this subscript has type parameters
    pub fn has_type_parameters(&self) -> bool {
        self.syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::TypeParameterList)
    }

    /// Check if this subscript has a where clause
    pub fn has_where_clause(&self) -> bool {
        self.syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::WhereClause)
    }
}

/// Parse a subscript declaration and emit events
///
/// This is the primary event-driven parser function for subscript declarations.
pub fn parse_subscript_declaration<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    use chumsky::Parser;

    let prepared = prepare_tokens(tokens);
    let input = create_input(&prepared, source.len());

    match subscript_declaration_parser_internal()
        .parse(input)
        .into_result()
    {
        Ok(data) => {
            emit_subscript_declaration(sink, data);
        }
        Err(errors) => {
            for error in errors {
                let span = error.span();
                sink.error_at(format!("Parse error: {:?}", error), *span);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_lexer::lex;

    #[test]
    fn test_subscript_declaration_shorthand() {
        let source = "subscript(index: Int) -> T { self.data(index) }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_subscript_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = SubscriptDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.visibility(), None);
        assert!(!decl.is_static());
        assert!(decl.parameter_list().is_some());
        assert!(decl.return_type().is_some());
        assert!(decl.body().is_some());
        assert!(decl.is_getter_only());
    }

    #[test]
    fn test_subscript_declaration_with_visibility() {
        let source = "public subscript(index: Int) -> T { self.data(index) }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_subscript_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = SubscriptDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.visibility(), Some(SyntaxKind::Public));
    }

    #[test]
    fn test_subscript_declaration_static() {
        let source = "static subscript(key: String) -> T { Self.storage(key) }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_subscript_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = SubscriptDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert!(decl.is_static());
    }

    #[test]
    fn test_subscript_declaration_labeled_param() {
        let source = "subscript(safe index: Int) -> T { self.data(index) }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_subscript_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = SubscriptDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert!(decl.parameter_list().is_some());
    }

    #[test]
    fn test_subscript_declaration_with_generics() {
        let source = "subscript[K](key: K) -> V where K: Hashable { self.lookup(key) }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_subscript_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = SubscriptDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert!(decl.has_type_parameters());
        assert!(decl.has_where_clause());
    }

    #[test]
    fn test_subscript_declaration_explicit_get_set() {
        let source = "subscript(index: Int) -> T { get { self.data(index) } set { self.data(index) = newValue } }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_subscript_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = SubscriptDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert!(!decl.is_getter_only());
        assert!(decl.getter_clause().is_some());
        assert!(decl.setter_clause().is_some());
    }

    #[test]
    fn test_subscript_declaration_protocol_get() {
        let source = "subscript(index: Int) -> T { get }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_subscript_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = SubscriptDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert!(decl.is_protocol_requirement());
        assert!(decl.is_getter_only());
    }

    #[test]
    fn test_subscript_declaration_protocol_get_set() {
        let source = "subscript(index: Int) -> T { get set }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_subscript_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = SubscriptDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert!(decl.is_protocol_requirement());
        assert!(!decl.is_getter_only());
    }

    #[test]
    fn test_subscript_declaration_full() {
        let source = "public static subscript[T](index: Int) -> T where T: Copyable { self.data(index) }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_subscript_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = SubscriptDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.visibility(), Some(SyntaxKind::Public));
        assert!(decl.is_static());
        assert!(decl.has_type_parameters());
        assert!(decl.has_where_clause());
        assert!(decl.parameter_list().is_some());
        assert!(decl.return_type().is_some());
    }
}
