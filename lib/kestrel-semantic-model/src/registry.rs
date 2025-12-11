//! Symbol registry for O(1) symbol lookup
//!
//! The registry stores all symbols in the semantic tree and provides
//! efficient lookup by ID or by (kind, name) pairs.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;

use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::{Symbol, SymbolId};

/// Thread-safe registry of all symbols in the tree
#[derive(Debug, Clone)]
pub struct SymbolRegistry {
    symbols: Arc<RwLock<HashMap<SymbolId, Arc<dyn Symbol<KestrelLanguage>>>>>,
    /// Index for O(1) lookup of symbols by (kind, name)
    /// Used primarily for module path resolution
    kind_name_index: Arc<RwLock<HashMap<(KestrelSymbolKind, String), Vec<SymbolId>>>>,
}

impl SymbolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            symbols: Arc::new(RwLock::new(HashMap::new())),
            kind_name_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a single symbol
    pub fn register(&self, symbol: Arc<dyn Symbol<KestrelLanguage>>) {
        let id = symbol.metadata().id();
        let kind = symbol.metadata().kind();
        let name = symbol.metadata().name().value.clone();

        self.symbols.write().insert(id, symbol);

        // Add to kind+name index for O(1) lookups
        self.kind_name_index
            .write()
            .entry((kind, name))
            .or_insert_with(Vec::new)
            .push(id);
    }

    /// Get symbol by ID
    pub fn get(&self, id: SymbolId) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        self.symbols.read().get(&id).cloned()
    }

    /// Register entire symbol tree recursively
    pub fn register_tree(&self, root: &Arc<dyn Symbol<KestrelLanguage>>) {
        self.register(root.clone());
        for child in root.metadata().children() {
            self.register_tree(&child);
        }
    }

    /// Get total number of registered symbols
    pub fn len(&self) -> usize {
        self.symbols.read().len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.symbols.read().is_empty()
    }

    /// Iterate over all symbols
    pub fn iter(&self) -> impl Iterator<Item = (SymbolId, Arc<dyn Symbol<KestrelLanguage>>)> + '_ {
        SymbolRegistryIter {
            guard: self.symbols.read(),
            keys: None,
        }
    }

    /// Look up symbols by kind and name in O(1) time
    pub fn find_by_kind_and_name(
        &self,
        kind: KestrelSymbolKind,
        name: &str,
    ) -> Vec<Arc<dyn Symbol<KestrelLanguage>>> {
        let index = self.kind_name_index.read();
        let symbols = self.symbols.read();

        index
            .get(&(kind, name.to_string()))
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| symbols.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Default for SymbolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Iterator over symbol registry
struct SymbolRegistryIter<'a> {
    guard: parking_lot::RwLockReadGuard<'a, HashMap<SymbolId, Arc<dyn Symbol<KestrelLanguage>>>>,
    keys: Option<Vec<SymbolId>>,
}

impl<'a> Iterator for SymbolRegistryIter<'a> {
    type Item = (SymbolId, Arc<dyn Symbol<KestrelLanguage>>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.keys.is_none() {
            self.keys = Some(self.guard.keys().copied().collect());
        }
        let keys = self.keys.as_mut()?;
        let id = keys.pop()?;
        self.guard.get(&id).map(|s| (id, s.clone()))
    }
}
