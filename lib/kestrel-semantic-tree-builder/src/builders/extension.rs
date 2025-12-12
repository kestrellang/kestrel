use std::sync::Arc;

use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::extension::ExtensionSymbol;
use kestrel_syntax_tree::SyntaxNode;
use semantic_tree::symbol::Symbol;

use kestrel_syntax_tree::utils::get_node_span;
use crate::builder::Builder;

/// Builder for extension declarations.
pub struct ExtensionBuilder;

impl Builder for ExtensionBuilder {
    fn build_declaration(
        &self,
        syntax: &SyntaxNode,
        source: &str,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        _root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        let full_span = get_node_span(syntax, source);

        let extension_symbol = ExtensionSymbol::new(full_span.clone(), parent.cloned());
        let extension_arc = Arc::new(extension_symbol);
        let extension_arc_dyn = extension_arc.clone() as Arc<dyn Symbol<KestrelLanguage>>;

        if let Some(parent) = parent {
            parent.metadata().add_child(&extension_arc_dyn);
        }

        Some(extension_arc)
    }
}
