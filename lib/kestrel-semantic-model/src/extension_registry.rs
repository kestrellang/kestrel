//! Extension registry for tracking extensions by target type
//!
//! The registry maps target struct SymbolIds to the extensions that extend them.
//! This enables O(1) lookup of all extensions for a given type during method resolution.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use semantic_tree::symbol::SymbolId;

use kestrel_semantic_tree::symbol::extension::ExtensionSymbol;
use semantic_tree::symbol::Symbol;

/// Thread-safe registry mapping target types to their extensions
#[derive(Debug, Clone)]
pub struct ExtensionRegistry {
    /// Maps target struct SymbolId -> list of extension SymbolIds
    extensions_by_target: Arc<RwLock<HashMap<SymbolId, Vec<SymbolId>>>>,

    /// All registered extension symbols (for full lookup)
    extensions: Arc<RwLock<HashMap<SymbolId, Arc<ExtensionSymbol>>>>,
}

impl ExtensionRegistry {
    /// Create a new empty extension registry
    pub fn new() -> Self {
        Self {
            extensions_by_target: Arc::new(RwLock::new(HashMap::new())),
            extensions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register an extension for a target type
    ///
    /// # Arguments
    /// * `target_id` - The SymbolId of the struct being extended
    /// * `extension` - The extension symbol
    pub fn register(&self, target_id: SymbolId, extension: Arc<ExtensionSymbol>) {
        let extension_id = extension.metadata().id();

        // Store the extension
        self.extensions.write().insert(extension_id, extension);

        // Add to target index
        self.extensions_by_target
            .write()
            .entry(target_id)
            .or_insert_with(Vec::new)
            .push(extension_id);
    }

    /// Get all extensions for a target type
    pub fn get_extensions_for(&self, target_id: SymbolId) -> Vec<Arc<ExtensionSymbol>> {
        let by_target = self.extensions_by_target.read();
        let extensions = self.extensions.read();

        by_target
            .get(&target_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| extensions.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get an extension by its ID
    pub fn get(&self, id: SymbolId) -> Option<Arc<ExtensionSymbol>> {
        self.extensions.read().get(&id).cloned()
    }

    /// Get all registered extensions
    pub fn all_extensions(&self) -> Vec<Arc<ExtensionSymbol>> {
        self.extensions.read().values().cloned().collect()
    }

    /// Get total number of registered extensions
    pub fn len(&self) -> usize {
        self.extensions.read().len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.extensions.read().is_empty()
    }

    /// Get all target type IDs that have extensions
    pub fn extended_types(&self) -> Vec<SymbolId> {
        self.extensions_by_target.read().keys().copied().collect()
    }
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self::new()
    }
}
