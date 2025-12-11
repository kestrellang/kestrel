//! Query system for semantic analysis and scope resolution
//!
//! This module provides a query-based API for resolving symbols and scopes.
//! Queries are memoized for performance (can be upgraded to Salsa later).

use std::sync::Arc;

use kestrel_semantic_tree::error::ModuleNotFoundError;
use kestrel_semantic_tree::language::KestrelLanguage;
use semantic_tree::symbol::{Symbol, SymbolId};

// Re-export types from kestrel-semantic-model for backwards compatibility
pub use kestrel_semantic_model::{
    Import, ImportItem, Scope, SymbolResolution, TypePathResolution,
    ValuePathResolution,
};

/// Database trait for semantic queries
pub trait Db {
    /// Get symbol by ID from the registry
    fn symbol_by_id(&self, id: SymbolId) -> Option<Arc<dyn Symbol<KestrelLanguage>>>;

    /// Get the scope for a symbol
    fn scope_for(&self, symbol_id: SymbolId) -> Arc<Scope>;

    /// Resolve a name in a given scope context
    fn resolve_name(&self, name: String, context: SymbolId) -> SymbolResolution;

    /// Get all imports declared in a symbol's scope
    fn imports_in_scope(&self, symbol_id: SymbolId) -> Vec<Arc<Import>>;

    /// Check if target is visible from context
    fn is_visible_from(&self, target: SymbolId, context: SymbolId) -> bool;

    /// Resolve a module path from a context
    fn resolve_module_path(
        &self,
        path: Vec<String>,
        context: SymbolId,
    ) -> Result<SymbolId, ModuleNotFoundError>;

    /// Resolve a type path (e.g., "Foo.Bar.Baz") to a Type
    fn resolve_type_path(&self, path: Vec<String>, context: SymbolId) -> TypePathResolution;

    /// Resolve a value path (e.g., "module.function" or "x") to a value
    fn resolve_value_path(&self, path: Vec<String>, context: SymbolId) -> ValuePathResolution;

    /// Get visible children of a symbol that are visible from the given context
    fn visible_children_from(
        &self,
        parent: SymbolId,
        context: SymbolId,
    ) -> Vec<Arc<dyn Symbol<KestrelLanguage>>>;

    /// Find a child symbol by name (without visibility check)
    fn find_child_by_name(
        &self,
        parent: SymbolId,
        name: &str,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>>;

    /// Register an extension for a target type
    fn register_extension(
        &self,
        target_id: SymbolId,
        extension: Arc<kestrel_semantic_tree::symbol::extension::ExtensionSymbol>,
    );

    /// Get all extensions registered for a target type
    fn get_extensions_for(
        &self,
        target_id: SymbolId,
    ) -> Vec<Arc<kestrel_semantic_tree::symbol::extension::ExtensionSymbol>>;
}
