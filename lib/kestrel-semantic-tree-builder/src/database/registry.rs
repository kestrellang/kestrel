//! Symbol registry for O(1) symbol lookup
//!
//! The registry stores all symbols in the semantic tree and provides
//! efficient lookup by ID or by (kind, name) pairs.

// Re-export from kestrel-semantic-model
pub use kestrel_semantic_model::SymbolRegistry;
