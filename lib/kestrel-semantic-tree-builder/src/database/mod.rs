//! Semantic database and symbol registry
//!
//! This module re-exports types from kestrel-semantic-model for convenience.

// Re-export types from kestrel-semantic-model
pub use kestrel_semantic_model::{
    ExtensionRegistry, Import, ImportItem, Scope, SymbolRegistry, SymbolResolution,
    TypePathResolution, ValuePathResolution,
};
