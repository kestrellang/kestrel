//! Extension registry for tracking extensions by target type
//!
//! The registry maps target struct SymbolIds to the extensions that extend them.
//! This enables O(1) lookup of all extensions for a given type during method resolution.

// Re-export from kestrel-semantic-model
pub use kestrel_semantic_model::ExtensionRegistry;
