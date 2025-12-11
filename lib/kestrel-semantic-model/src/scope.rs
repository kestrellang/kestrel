//! Scope and import types for semantic analysis
//!
//! This module provides types for representing lexical scopes and import declarations.

use std::collections::HashMap;

use semantic_tree::symbol::SymbolId;

/// Scope information for a symbol
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    /// The symbol this scope belongs to
    pub symbol_id: SymbolId,
    /// Imported names -> symbol IDs
    pub imports: HashMap<String, Vec<SymbolId>>,
    /// Declared names -> symbol IDs
    pub declarations: HashMap<String, Vec<SymbolId>>,
    /// Parent scope for lookup chain
    pub parent: Option<SymbolId>,
}

/// Import metadata (extracted from ImportDataBehavior)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Import {
    /// The module path (e.g., ["A", "B", "C"])
    pub module_path: Vec<String>,
    /// Optional alias for the module
    pub alias: Option<String>,
    /// Specific items to import
    pub items: Vec<ImportItem>,
}

/// An individual import item
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImportItem {
    /// Name of the symbol to import
    pub name: String,
    /// Optional alias for this import
    pub alias: Option<String>,
    /// Resolved target symbol ID (filled during bind)
    pub target_id: Option<SymbolId>,
}
