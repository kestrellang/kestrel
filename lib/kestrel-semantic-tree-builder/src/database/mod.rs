//! Semantic database and symbol registry
//!
//! This module provides the database layer for semantic analysis:
//! - `SymbolRegistry`: Thread-safe storage and indexing of symbols
//! - `ExtensionRegistry`: Tracks extensions by target type
//! - `SemanticDatabase`: Query interface with caching
//! - `Db` trait: Query interface for semantic analysis

mod extension_registry;
mod registry;
pub mod queries;
mod semantic_db;

pub use extension_registry::ExtensionRegistry;
pub use registry::SymbolRegistry;
pub use queries::{
    Db, Import, ImportItem, Scope, SymbolResolution, TypePathResolution, ValuePathResolution,
};
pub use semantic_db::SemanticDatabase;

// Re-export the helper function
pub use queries::get_import_data;
