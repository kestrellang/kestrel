use std::sync::Arc;

use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_syntax_tree::SyntaxNode;
use semantic_tree::symbol::Symbol;

use crate::resolver::Resolver;

/// Resolver for module declarations
///
/// Module declarations are handled specially during semantic tree building.
/// They define the namespace hierarchy but aren't created as symbols during
/// the normal tree walk. Instead, the module hierarchy is built before processing
/// other declarations. This resolver returns None to skip module declarations
/// during the normal walk.
pub struct ModuleResolver;

impl Resolver for ModuleResolver {
    fn build_declaration(
        &self,
        _syntax: &SyntaxNode,
        _source: &str,
        _parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        _root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        // Module declarations are processed separately during tree building
        // Return None to skip during normal walk
        None
    }

    fn is_terminal(&self) -> bool {
        // Module declarations don't have children to walk
        true
    }
}
