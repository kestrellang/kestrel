use std::sync::Arc;

use kestrel_parser::import::ImportDeclaration;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::import::{ImportDataBehavior, ImportItem, ImportSymbol};
use kestrel_span::{Span, Spanned};
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use crate::builder::Builder;
use kestrel_syntax_tree::utils::get_node_span;

/// Builder for import declarations.
pub struct ImportBuilder;

impl Builder for ImportBuilder {
    fn build_declaration(
        &self,
        syntax: &SyntaxNode,
        source: &str,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        _root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        let parent = parent?;

        let import_decl = ImportDeclaration {
            syntax: syntax.clone(),
            span: get_node_span(syntax, source),
        };

        let module_path_node = import_decl.path();
        let module_path_segments = module_path_node.segments_with_spans();
        let module_path_span = module_path_node.span();

        let alias = import_decl.alias();
        let items = extract_import_items(&import_decl);

        let import_name = if let Some(ref alias) = alias {
            alias.clone()
        } else {
            module_path_segments
                .iter()
                .map(|(s, _)| s.as_str())
                .collect::<Vec<_>>()
                .join(".")
        };

        let span = get_node_span(syntax, source);
        let name = Spanned::new(import_name, span.clone());

        let import_symbol = ImportSymbol::new(name, parent.clone(), span);
        let import_arc: Arc<dyn Symbol<KestrelLanguage>> = Arc::new(import_symbol);

        let import_data =
            ImportDataBehavior::new(module_path_segments, module_path_span, alias, items);
        import_arc.metadata().add_behavior(import_data);

        parent.metadata().add_child(&import_arc);

        Some(import_arc)
    }

    fn is_terminal(&self) -> bool {
        true
    }
}

fn extract_import_items(import_decl: &ImportDeclaration) -> Vec<ImportItem> {
    import_decl
        .items()
        .into_iter()
        .filter_map(|item_node| {
            let (name, span) = item_node.children_with_tokens().find_map(|elem| {
                elem.as_token()
                    .filter(|t| t.kind() == SyntaxKind::Identifier)
                    .map(|t| {
                        let range = t.text_range();
                        let span: Span = Span::from(range.start().into()..range.end().into());
                        (t.text().to_string(), span)
                    })
            })?;

            let mut found_as = false;
            let alias = item_node.children_with_tokens().find_map(|elem| {
                if let Some(token) = elem.as_token() {
                    if found_as && token.kind() == SyntaxKind::Identifier {
                        return Some(token.text().to_string());
                    }
                    if token.kind() == SyntaxKind::As {
                        found_as = true;
                    }
                }
                None
            });

            Some(ImportItem {
                name,
                alias,
                span,
                target_id: None,
            })
        })
        .collect()
}
