//! Semantic database and symbol registry
//!
//! This module provides the database layer for semantic analysis:
//! - `SymbolRegistry`: Thread-safe storage and indexing of symbols
//! - `ExtensionRegistry`: Tracks extensions by target type
//! - `SemanticDatabase`: Query interface with caching
//! - `Db` trait: Query interface for semantic analysis

pub mod queries;
mod semantic_db;

// Re-export types from kestrel-semantic-model
pub use kestrel_semantic_model::{
    ExtensionRegistry, Import, ImportItem, Scope, SymbolRegistry, SymbolResolution,
    TypePathResolution, ValuePathResolution, get_import_data,
};

pub use queries::Db;
pub use semantic_db::SemanticDatabase;
