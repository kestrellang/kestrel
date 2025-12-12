use std::sync::Arc;

use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_syntax_tree::SyntaxNode;
use semantic_tree::symbol::Symbol;

use crate::resolver::Resolver;

/// Terminal resolver for wrapper nodes that don't produce symbols
/// Used for nodes like Visibility and Name that should stop tree traversal
pub struct TerminalResolver;

impl Resolver for TerminalResolver {
    fn build_declaration(
        &self,
        _syntax: &SyntaxNode,
        _source: &str,
        _parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        _root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        // Terminal nodes don't produce symbols
        None
    }

    fn is_terminal(&self) -> bool {
        true
    }
}
