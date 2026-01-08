use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::common::{emit_import_declaration, import_declaration_parser_internal};
use crate::event::EventSink;
use crate::input::{create_input, prepare_tokens};
use crate::module::ModulePath;

/// Represents an import declaration
///
/// The declaration is stored as a lossless syntax tree. All data is derived
/// from the tree rather than stored separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportDeclaration {
    pub syntax: SyntaxNode,
    pub span: Span,
}

impl ImportDeclaration {
    /// Get the module path from this import declaration
    pub fn path(&self) -> ModulePath {
        self.syntax
            .children()
            .find(|node| node.kind() == SyntaxKind::ModulePath)
            .map(|node| ModulePath { syntax: node })
            .expect("ImportDeclaration must have a ModulePath child")
    }

    /// Check if this is an "import all" declaration (e.g., `import A.B.C`)
    pub fn is_import_all(&self) -> bool {
        // If there's no "as" keyword and no items list, it's import all
        !self.has_alias() && !self.has_items()
    }

    /// Check if this import has an alias (e.g., `import A.B.C as D`)
    pub fn has_alias(&self) -> bool {
        self.syntax.children_with_tokens().any(|elem| {
            elem.as_token()
                .map(|t| t.kind() == SyntaxKind::As)
                .unwrap_or(false)
        }) && !self.has_items()
    }

    /// Check if this import has an items list (e.g., `import A.B.C.(D, E)`)
    pub fn has_items(&self) -> bool {
        self.syntax.children_with_tokens().any(|elem| {
            elem.as_token()
                .map(|t| t.kind() == SyntaxKind::LParen)
                .unwrap_or(false)
        })
    }

    /// Get the alias identifier if present (for `import A.B.C as D`)
    pub fn alias(&self) -> Option<String> {
        if !self.has_alias() {
            return None;
        }

        // Find the identifier after the "as" keyword
        let mut found_as = false;
        for elem in self.syntax.children_with_tokens() {
            if let Some(token) = elem.as_token() {
                if found_as && token.kind() == SyntaxKind::Identifier {
                    return Some(token.text().to_string());
                }
                if token.kind() == SyntaxKind::As {
                    found_as = true;
                }
            }
        }
        None
    }

    /// Get the import items if present (for `import A.B.C.(D, E)`)
    pub fn items(&self) -> Vec<SyntaxNode> {
        self.syntax
            .children()
            .filter(|node| node.kind() == SyntaxKind::ImportItem)
            .collect()
    }
}

/// Parse an import declaration and emit events
/// This is the primary event-driven parser function
pub fn parse_import_declaration<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    use chumsky::prelude::*;

    let prepared = prepare_tokens(tokens);
    let input = create_input(&prepared, source.len());

    match import_declaration_parser_internal()
        .parse(input)
        .into_result()
    {
        Ok((import_span, path_segments, alias, items)) => {
            emit_import_declaration(sink, import_span, &path_segments, alias, items);
        }
        Err(errors) => {
            // Emit error events for each parse error
            for error in errors {
                // Chumsky errors have span information
                let span = error.span();
                sink.error_at(format!("Parse error: {:?}", error), *span);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::TreeBuilder;
    use kestrel_lexer::lex;

    #[test]
    fn test_import_all() {
        let source = "import A.B.C";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_import_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = ImportDeclaration {
            syntax: tree,
            span: Span::from(0..source.len()),
        };

        assert_eq!(decl.path().segment_names(), vec!["A", "B", "C"]);
        assert!(decl.is_import_all());
        assert_eq!(decl.syntax.kind(), SyntaxKind::ImportDeclaration);
    }

    #[test]
    fn test_import_aliased() {
        let source = "import A.B.C as D";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_import_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = ImportDeclaration {
            syntax: tree,
            span: Span::from(0..source.len()),
        };

        assert_eq!(decl.path().segment_names(), vec!["A", "B", "C"]);
        assert!(decl.has_alias());
        assert_eq!(decl.alias(), Some("D".to_string()));
    }

    #[test]
    fn test_import_items() {
        let source = "import A.B.C.(D, E)";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_import_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = ImportDeclaration {
            syntax: tree,
            span: Span::from(0..source.len()),
        };

        assert_eq!(decl.path().segment_names(), vec!["A", "B", "C"]);
        assert!(decl.has_items());

        let items = decl.items();
        assert_eq!(items.len(), 2);

        // Check first item (D)
        let first_id = items[0]
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::Identifier)
            .unwrap();
        assert_eq!(first_id.text(), "D");

        // Check second item (E)
        let second_id = items[1]
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::Identifier)
            .unwrap();
        assert_eq!(second_id.text(), "E");
    }

    #[test]
    fn test_import_aliased_items() {
        let source = "import A.B.C.(D as E, F as G)";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_import_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = ImportDeclaration {
            syntax: tree,
            span: Span::from(0..source.len()),
        };

        assert_eq!(decl.path().segment_names(), vec!["A", "B", "C"]);
        assert!(decl.has_items());

        let items = decl.items();
        assert_eq!(items.len(), 2);

        // Check first item (D as E)
        let first_tokens: Vec<_> = items[0]
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| t.kind() == SyntaxKind::Identifier)
            .collect();
        assert_eq!(first_tokens.len(), 2);
        assert_eq!(first_tokens[0].text(), "D");
        assert_eq!(first_tokens[1].text(), "E");

        // Check second item (F as G)
        let second_tokens: Vec<_> = items[1]
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| t.kind() == SyntaxKind::Identifier)
            .collect();
        assert_eq!(second_tokens.len(), 2);
        assert_eq!(second_tokens[0].text(), "F");
        assert_eq!(second_tokens[1].text(), "G");
    }
}
