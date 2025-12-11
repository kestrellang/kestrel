//! Semantic model types for Kestrel compiler
//!
//! This crate provides foundational types for semantic analysis including:
//! - Scope and import representation
//! - Resolution result types
//! - Symbol and extension registries
//! - SemanticModel for querying semantic information

mod extension_registry;
mod model;
mod query;
mod registry;
mod resolution;
mod scope;

pub use extension_registry::ExtensionRegistry;
pub use model::SemanticModel;
pub use query::Query;
pub use registry::SymbolRegistry;
pub use resolution::{SymbolResolution, TypePathResolution, ValuePathResolution};
pub use scope::{Import, ImportItem, Scope};

use std::sync::Arc;

use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::import::ImportDataBehavior;
use semantic_tree::symbol::Symbol;

/// Helper to get ImportDataBehavior from a symbol
pub fn get_import_data(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> Option<Arc<ImportDataBehavior>> {
    symbol
        .metadata()
        .behaviors()
        .into_iter()
        .find(|b| matches!(b.kind(), KestrelBehaviorKind::ImportData))
        .and_then(|b| {
            b.as_ref()
                .downcast_ref::<ImportDataBehavior>()
                .map(|data| Arc::new(data.clone()))
        })
}
