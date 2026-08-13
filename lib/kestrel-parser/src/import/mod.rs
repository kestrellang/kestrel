use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::common::parsers::ModulePathSpans;
use crate::common::{emit_module_path, identifier, module_path_parser_internal, token};
use crate::event::EventSink;
use crate::input::{ParserExtra, ParserInput};
use crate::module::ModulePath;
use crate::parse_and_emit;

use chumsky::prelude::*;

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
    parse_and_emit!(
        source,
        tokens,
        sink,
        import_declaration_parser_internal(),
        |sink, import: ImportSpans| emit_import_declaration(sink, &import)
    );
}

/// One entry of an import list: `Name` or `Name as Alias`.
///
/// The `as` keyword's span is carried, not reconstructed as
/// `name.end + 1 .. name.end + 3` — `token()` skips trivia, so `Name   as X`
/// and a line break before `as` are both grammatical and the arithmetic span
/// landed on whitespace (F25).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportItemSpans {
    pub name: Span,
    /// `(as_keyword, alias_identifier)` when the item is aliased.
    pub alias: Option<(Span, Span)>,
}

/// A parenthesised import list with every punctuation span the CST needs.
///
/// `lparen` / `rparen` / `commas` used to be invented by byte arithmetic off
/// the neighbouring identifiers. `lang/std/numeric/int64.ks` is a multi-line
/// `import std.core.(\n … \n)`, so the fabricated `RParen` range held a
/// newline and the real `)` came out as a `SyntaxKind::Error` token — the
/// documented *recovery* marker, emitted for well-formed shipping source (F25).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportItemsSpans {
    pub lparen: Span,
    pub items: Vec<ImportItemSpans>,
    /// Separator `,`s. Always `items.len() - 1` of them — the grammar has no
    /// trailing comma.
    pub commas: Vec<Span>,
    pub rparen: Span,
}

/// Internal parser for import item (identifier or identifier as alias).
fn import_item_parser_internal<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, ImportItemSpans, ParserExtra<'tokens>> + Clone {
    identifier()
        .then(token(Token::As).then(identifier()).or_not())
        .map(|(name, alias)| ImportItemSpans { name, alias })
        .boxed()
}

/// Internal parser for import items list.
fn import_items_parser_internal<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, ImportItemsSpans, ParserExtra<'tokens>> + Clone {
    token(Token::LParen)
        .then(
            // No trailing comma: `separated_by` did not permit one, and this
            // rewrite only changes which SPANS are recorded, never the grammar.
            import_item_parser_internal().then(
                token(Token::Comma)
                    .then(import_item_parser_internal())
                    .repeated()
                    .collect::<Vec<(Span, ImportItemSpans)>>(),
            ),
        )
        .then(token(Token::RParen))
        .map(|((lparen, (first, rest)), rparen)| {
            let mut items = vec![first];
            let mut commas = Vec::with_capacity(rest.len());
            for (comma, item) in rest {
                commas.push(comma);
                items.push(item);
            }
            ImportItemsSpans {
                lparen,
                items,
                commas,
                rparen,
            }
        })
        .boxed()
}

/// What follows the module path in an `import`: nothing, `as Alias`, or
/// `.(Item, …)`. Every token span the CST needs is carried here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportTail {
    /// `import A.B as C` — `(as_keyword, alias_identifier)`.
    Alias(Span, Span),
    /// `import A.B.(X, Y)` — `(dot_before_paren, items)`.
    Items(Span, ImportItemsSpans),
}

/// Everything an `import` declaration needs to emit a faithful CST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSpans {
    pub import_kw: Span,
    pub path: ModulePathSpans,
    pub tail: Option<ImportTail>,
}

/// Internal Chumsky parser for import declarations.
pub(crate) fn import_declaration_parser_internal<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, ImportSpans, ParserExtra<'tokens>> + Clone {
    token(Token::Import)
        .then(module_path_parser_internal())
        .then(
            token(Token::As)
                .then(identifier())
                .map(|(as_kw, alias)| ImportTail::Alias(as_kw, alias))
                .or(token(Token::Dot)
                    .then(import_items_parser_internal())
                    .map(|(dot, items)| ImportTail::Items(dot, items)))
                .or_not(),
        )
        .map(|((import_kw, path), tail)| ImportSpans {
            import_kw,
            path,
            tail,
        })
        .boxed()
}

/// Emit events for an import declaration.
///
/// Every token comes from a span the parser captured. This function used to
/// reconstruct the `.`, `(`, `,`, `as` and `)` by byte arithmetic off the
/// neighbouring identifiers — assuming exactly one byte of separator and zero
/// trivia. `TreeBuilder` then took the token text from the fabricated range,
/// hit its non-trivia safety net, and emitted the real punctuation as
/// `SyntaxKind::Error` (F25).
pub(crate) fn emit_import_declaration(sink: &mut EventSink, import: &ImportSpans) {
    sink.start_node(SyntaxKind::ImportDeclaration);
    sink.add_token(SyntaxKind::Import, import.import_kw.clone());

    emit_module_path(sink, &import.path);

    match &import.tail {
        Some(ImportTail::Items(dot, list)) => {
            sink.add_token(SyntaxKind::Dot, dot.clone());
            sink.add_token(SyntaxKind::LParen, list.lparen.clone());

            for (i, item) in list.items.iter().enumerate() {
                if i > 0 {
                    sink.add_token(SyntaxKind::Comma, list.commas[i - 1].clone());
                }
                sink.start_node(SyntaxKind::ImportItem);
                sink.add_token(SyntaxKind::Identifier, item.name.clone());
                if let Some((as_kw, alias)) = &item.alias {
                    sink.add_token(SyntaxKind::As, as_kw.clone());
                    sink.add_token(SyntaxKind::Identifier, alias.clone());
                }
                sink.finish_node();
            }

            sink.add_token(SyntaxKind::RParen, list.rparen.clone());
        },
        Some(ImportTail::Alias(as_kw, alias)) => {
            sink.add_token(SyntaxKind::As, as_kw.clone());
            sink.add_token(SyntaxKind::Identifier, alias.clone());
        },
        None => {},
    }

    sink.finish_node();
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
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.path().segment_names(), vec!["A", "B", "C"]);
        assert!(decl.is_import_all());
        assert_eq!(decl.syntax.kind(), SyntaxKind::ImportDeclaration);
    }

    /// Build the CST for `source` and return every token whose kind is `Error`,
    /// with its text.
    fn error_tokens(source: &str) -> Vec<(String, std::ops::Range<usize>)> {
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();
        let mut sink = EventSink::new(0);
        parse_import_declaration(source, tokens.into_iter(), &mut sink);
        let tree = TreeBuilder::new(source, sink.into_events()).build();

        fn walk(
            node: &kestrel_syntax_tree::SyntaxNode,
            out: &mut Vec<(String, std::ops::Range<usize>)>,
        ) {
            for elem in node.descendants_with_tokens() {
                if let Some(tok) = elem.as_token()
                    && tok.kind() == SyntaxKind::Error
                {
                    let r = tok.text_range();
                    out.push((
                        tok.text().to_string(),
                        usize::from(r.start())..usize::from(r.end()),
                    ));
                }
            }
        }
        let mut out = Vec::new();
        walk(&tree, &mut out);
        out
    }

    /// `SyntaxKind::Error` is the documented parse-**recovery** marker. Emitting
    /// it for well-formed source violates the architecture doc's "preserve
    /// source token order and source spans" contract, and it did: every
    /// punctuation span in an import was reconstructed by byte arithmetic off a
    /// neighbouring identifier, assuming one byte of separator and zero trivia.
    /// `token()` skips trivia, so these three shapes are all grammatical and
    /// all produced fabricated ranges holding whitespace, with the real
    /// punctuation re-emitted as `Error` by the tree builder's safety net (F25).
    ///
    /// `lang/std/numeric/int64.ks` is the multi-line form, so this fired on
    /// shipping stdlib source on every build.
    #[test]
    fn well_formed_imports_produce_no_error_tokens() {
        for source in [
            "import A.B.C",
            "import A.B.C as D",
            "import A.B.(X, Y)",
            // Space before the paren and around the dots.
            "import A . B . (X, Y)",
            // Space around `as`, which the `name.end + 1 .. + 3` span missed.
            "import A.B.(X  as  Y)",
            // The stdlib shape: multi-line list, so the `)` span was a newline.
            "import std.core.(\n    Int64,\n    String\n)",
            "import std.core.(\n    Int64 as I,\n    String as S\n)",
        ] {
            assert_eq!(
                error_tokens(source),
                vec![],
                "well-formed import produced Error token(s): {source:?}"
            );
        }
    }

    /// The CST must still be lossless: concatenating it reproduces the source
    /// byte for byte. This is what kept F25 invisible — the round trip passed
    /// while the token *kinds* were wrong — so it is asserted alongside, not
    /// instead of, the Error-token check.
    #[test]
    fn multiline_import_cst_round_trips_with_correct_punctuation() {
        let source = "import std.core.(\n    Int64,\n    String as Str\n)";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();
        let mut sink = EventSink::new(0);
        parse_import_declaration(source, tokens.into_iter(), &mut sink);
        let tree = TreeBuilder::new(source, sink.into_events()).build();

        assert_eq!(tree.text().to_string(), source);

        // The closing paren must be a real `)`, not the newline before it.
        let rparen = tree
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::RParen)
            .expect("import list must have an RParen token");
        assert_eq!(rparen.text(), ")");

        let commas: Vec<String> = tree
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| t.kind() == SyntaxKind::Comma)
            .map(|t| t.text().to_string())
            .collect();
        assert_eq!(commas, vec![","]);

        let as_kw = tree
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::As)
            .expect("aliased import item must have an As token");
        assert_eq!(as_kw.text(), "as");
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
            span: Span::new(0, 0..source.len()),
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
            span: Span::new(0, 0..source.len()),
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
            span: Span::new(0, 0..source.len()),
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
