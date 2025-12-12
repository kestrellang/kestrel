use std::sync::Arc;

use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_syntax_tree::SyntaxNode;
use semantic_tree::symbol::Symbol;

use crate::builder::Builder;

/// Builder for module declarations.
///
/// Module declarations are handled specially by the tree-building driver, not as symbols.
pub struct ModuleBuilder;

impl Builder for ModuleBuilder {
    fn build_declaration(
        &self,
        _syntax: &SyntaxNode,
        _source: &str,
        _parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        _root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        None
    }

    fn is_terminal(&self) -> bool {
        true
    }
}

