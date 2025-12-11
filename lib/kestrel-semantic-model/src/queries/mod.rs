//! Semantic model queries
//!
//! This module contains all query implementations for the SemanticModel.
//! Each query is a struct that implements the Query trait.

mod extensions_for;
mod scope_for;
mod symbol_for;

pub use extensions_for::ExtensionsFor;
pub use scope_for::ScopeFor;
pub use symbol_for::SymbolFor;
