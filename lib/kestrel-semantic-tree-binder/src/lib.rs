//! Semantic tree builder for the Kestrel language
//!
//! This crate provides the infrastructure for building semantic trees from syntax trees.
//! It handles:
//! - Building symbol hierarchies from syntax
//! - Resolving type references and imports
//! - Validating semantic constraints
//!
//! # Architecture
//!
//! The crate is organized into several modules:
//!
//! - `builder`: Tree construction from syntax (`SemanticTreeBuilder`)
//! - `database`: Re-exports from `kestrel-semantic-model` for convenience
//! - `resolution`: Binding and type resolution (`SemanticBinder`, `TypeResolver`)
//! - `syntax`: Syntax tree utilities
//! - `diagnostics`: Error types for semantic analysis
//! - `resolvers`: Per-declaration resolvers
//! - `body_resolver`: Function body resolution
//!
//! # Usage
//!
//! ```ignore
//! use kestrel_semantic_tree_binder::{SemanticTreeBuilder, SemanticBinder};
//!
//! // Build phase
//! let mut builder = SemanticTreeBuilder::new();
//! builder.add_file("main.kes", &syntax, &source, &mut diagnostics, 0);
//! let tree = builder.build();
//!
//! // Bind phase
//! let model = SemanticBinder::bind(tree, &mut diagnostics);
//! ```

// Core modules
pub mod builder;
mod debug;
pub mod resolution;
mod declaration_binder;
mod binders;
pub mod syntax;
mod tree;

// Feature modules
pub mod body_resolver;
pub mod diagnostics;

// Re-exports for convenient access
pub use builder::SemanticTreeBuilder;
pub use debug::{
    format_type, print_model_symbols, print_semantic_model, print_semantic_tree, print_symbol_table,
};
pub use kestrel_semantic_model::SemanticModel;
pub use resolution::{LocalScope, SemanticBinder, TypeResolver};
pub use declaration_binder::{BindingContext, DeclarationBinder, DeclarationBinderRegistry};
pub use tree::SemanticTree;
