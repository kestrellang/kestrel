//! Semantic tree building
//!
//! This module provides the `SemanticTreeBuilder` for constructing semantic trees
//! from syntax trees. The build phase creates symbol nodes and establishes the
//! parent-child hierarchy.

mod module_validator;
mod tree_builder;

pub use module_validator::{ModuleDeclaration, ModuleValidator};
pub use tree_builder::SemanticTreeBuilder;
