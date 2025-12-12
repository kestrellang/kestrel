use crate::language::Language;
use crate::symbol::{Symbol, SymbolCollection};
use std::collections::HashMap;
use std::sync::Arc;

/// A symbol table that maps names to collections of symbols.
///
/// This struct provides functionality for storing and retrieving symbols by name.
/// Multiple symbols can share the same name, which is useful for function overloading,
/// different scopes, or other cases where name collisions are expected.
///
/// # Examples
///
/// ```rust,ignore
/// let mut table = SymbolTable::new();
/// table.insert(symbol1);
/// table.insert(symbol2);
/// let symbols = table.get("function_name");
/// ```
#[derive(Debug, Clone)]
pub struct SymbolTable<L: Language> {
    symbols: HashMap<String, Vec<Arc<dyn Symbol<L>>>>,
}

impl<L: Language> SymbolTable<L> {
    /// Creates a new empty `SymbolTable`.
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
        }
    }

    /// Inserts a symbol into the table.
    /// If a symbol with the same name already exists, the new symbol is added to the collection.
    pub fn insert(&mut self, symbol: Arc<dyn Symbol<L>>) {
        let name = symbol.metadata().name().value;
        self.symbols.entry(name).or_default().push(symbol);
    }

    /// Retrieves all symbols with the given name, returning an empty collection if none exist.
    pub fn get(&self, name: &str) -> SymbolCollection<L> {
        let symbols = self.symbols.get(name).cloned().unwrap_or_default();
        SymbolCollection::new(symbols)
    }

    /// Returns `true` if the table contains any symbols with the given name.
    pub fn contains(&self, name: &str) -> bool {
        self.symbols.contains_key(name)
    }

    /// Returns the number of unique names in the table.
    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    /// Returns `true` if the table is empty.
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    /// Returns an iterator over all symbol collections in the table.
    pub fn iter(&self) -> impl Iterator<Item = (&String, SymbolCollection<L>)> {
        self.symbols
            .iter()
            .map(|(name, symbols)| (name, SymbolCollection::new(symbols.clone())))
    }

    /// Clears all symbols from the table.
    pub fn clear(&mut self) {
        self.symbols.clear();
    }
}

impl<L: Language> Default for SymbolTable<L> {
    fn default() -> Self {
        Self::new()
    }
}
