//! Semantic model for Kestrel compiler
//!
//! The SemanticModel is the primary interface for querying semantic information
//! about a compiled Kestrel program. It owns the symbol tree, syntax mappings,
//! source code, and registries.

use std::collections::HashMap;
use std::sync::Arc;

use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_syntax_tree::SyntaxNode;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::extension_registry::ExtensionRegistry;
use crate::query::Query;
use crate::registry::SymbolRegistry;
use kestrel_semantic_tree::symbol::extension::ExtensionSymbol;

/// The semantic model for a Kestrel program.
///
/// This is the primary interface for querying semantic information about a
/// compiled program. It owns the symbol tree, syntax mappings, source code,
/// and registries needed for semantic analysis.
pub struct SemanticModel {
    /// The root symbol of the semantic tree
    root: Arc<dyn Symbol<KestrelLanguage>>,
    /// Maps symbol IDs to their original syntax nodes
    syntax_map: HashMap<SymbolId, SyntaxNode>,
    /// Source code by filename
    sources: HashMap<String, String>,
    /// Symbol registry for O(1) lookup
    registry: SymbolRegistry,
    /// Extension registry for extension lookups
    extension_registry: ExtensionRegistry,
}

impl SemanticModel {
    /// Create a new SemanticModel from tree components.
    ///
    /// Called by SemanticBinder after the build phase.
    pub fn new(
        root: Arc<dyn Symbol<KestrelLanguage>>,
        syntax_map: HashMap<SymbolId, SyntaxNode>,
        sources: HashMap<String, String>,
    ) -> Self {
        let registry = SymbolRegistry::new();
        registry.register_tree(&root);

        Self {
            root,
            syntax_map,
            sources,
            registry,
            extension_registry: ExtensionRegistry::new(),
        }
    }

    /// Execute a query against this model.
    pub fn query<Q: Query>(&self, query: Q) -> Q::Output {
        query.execute(self)
    }

    /// Get the root symbol.
    pub fn root(&self) -> &Arc<dyn Symbol<KestrelLanguage>> {
        &self.root
    }

    /// Get the syntax node for a symbol.
    pub fn syntax_for(&self, symbol_id: SymbolId) -> Option<&SyntaxNode> {
        self.syntax_map.get(&symbol_id)
    }

    /// Get source code by filename.
    pub fn source(&self, filename: &str) -> Option<&str> {
        self.sources.get(filename).map(|s| s.as_str())
    }

    /// Get the symbol registry.
    ///
    /// Exposed for queries to access.
    pub(crate) fn registry(&self) -> &SymbolRegistry {
        &self.registry
    }

    /// Get the extension registry.
    ///
    /// Exposed for queries to access.
    pub(crate) fn extension_registry(&self) -> &ExtensionRegistry {
        &self.extension_registry
    }

    /// Register an extension (called during binding).
    pub fn register_extension(&self, target_id: SymbolId, extension: Arc<ExtensionSymbol>) {
        self.extension_registry.register(target_id, extension);
    }
}
