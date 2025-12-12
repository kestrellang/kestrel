//! Resolution and binding
//!
//! This module provides components for the bind phase of semantic analysis:
//! - `SemanticBinder`: Orchestrates binding of all symbols
//! - `TypeResolver`: Resolves types from syntax nodes
//! - `LocalScope`: Manages local variable scopes in function bodies

mod binder;
mod local_scope;
pub mod type_resolver;

pub use binder::SemanticBinder;
pub use local_scope::LocalScope;
pub use type_resolver::TypeResolver;
