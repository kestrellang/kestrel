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
//! - `database`: Query system for semantic analysis (`SemanticDatabase`, `Db` trait)
//! - `resolution`: Binding and type resolution (`SemanticBinder`, `TypeResolver`, `VisibilityChecker`)
//! - `syntax`: Syntax tree utilities
//! - `diagnostics`: Error types for semantic analysis
//! - `validation`: Validation passes
//! - `resolvers`: Per-declaration resolvers
//! - `body_resolver`: Function body resolution
//!
//! # Usage
//!
//! ```ignore
//! use kestrel_semantic_tree_builder::{SemanticTreeBuilder, SemanticBinder};
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
pub mod database;
mod debug;
pub mod resolution;
mod resolver;
mod resolvers;
pub mod syntax;
mod tree;

// Feature modules
pub mod body_resolver;
pub mod diagnostics;
pub mod validation;

// Re-exports for convenient access
pub use builder::SemanticTreeBuilder;
pub use database::{Db, SemanticDatabase, SymbolRegistry};
pub use debug::{format_type, print_semantic_tree, print_symbol_table, print_semantic_model, print_model_symbols};
pub use kestrel_semantic_model::SemanticModel;
pub use resolution::{LocalScope, SemanticBinder, TypeResolver, VisibilityChecker};
pub use resolver::{BindingContext, Resolver, ResolverRegistry};
pub use tree::SemanticTree;
