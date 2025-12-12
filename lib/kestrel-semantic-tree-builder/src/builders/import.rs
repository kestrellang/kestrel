use std::sync::Arc;

use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::import::{ImportDataBehavior, ImportItem, ImportSymbol};
use kestrel_span::Spanned;
use kestrel_syntax_tree::SyntaxNode;
use semantic_tree::symbol::Symbol;

use crate::builder::Builder;
use kestrel_syntax_tree::imports::extract_import_declaration;
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

        let import_syntax = extract_import_declaration(syntax)?;

        let module_path_segments = import_syntax.module_path;
        let module_path_span = import_syntax.module_path_span;

        let alias = import_syntax.alias;
        let items = import_syntax
            .items
            .into_iter()
            .map(|item| ImportItem {
                name: item.name,
                alias: item.alias,
                span: item.name_span,
                target_id: None,
            })
            .collect::<Vec<_>>();

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
