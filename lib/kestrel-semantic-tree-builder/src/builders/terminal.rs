use std::sync::Arc;

use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_syntax_tree::SyntaxNode;
use semantic_tree::symbol::Symbol;

use crate::builder::Builder;

/// Builder for wrapper/terminal nodes that don't produce symbols.
pub struct TerminalBuilder;

impl Builder for TerminalBuilder {
    fn build_declaration(
        &self,
        _syntax: &SyntaxNode,
        _source: &str,
        _file_id: usize,
        _parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        _root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        None
    }

    fn is_terminal(&self) -> bool {
        true
    }
}
