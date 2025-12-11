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
pub mod queries;
mod registry;
mod resolution;
mod scope;
mod visibility;

pub use extension_registry::ExtensionRegistry;
pub use model::SemanticModel;
pub use queries::{
    ChildByName, ExtensionsFor, ImportsInScope, IsVisibleFrom, ResolveModulePath, ResolveName,
    ResolveTypePath, ResolveValuePath, ScopeFor, SymbolFor, VisibleChildren,
};
pub use query::Query;
pub use registry::SymbolRegistry;
pub use resolution::{SymbolResolution, TypePathResolution, ValuePathResolution};
pub use scope::{Import, ImportItem, Scope};
