//! Semantic binder for the Kestrel language.
//!
//! This crate provides the bind phase of semantic analysis:
//! it resolves references and establishes relationships on a `SemanticModel`.
//!
//! # Architecture
//!
//! The crate is organized into several modules:
//!
//! - `resolution`: Binding and type resolution (`SemanticBinder`, `TypeResolver`)
//! - `syntax`: Syntax tree utilities
//! - `diagnostics`: Error types for semantic analysis
//! - `binders`: Per-declaration binders
//! - `body_resolver`: Function body resolution
//!
//! # Usage
//!
//! ```ignore
//! use kestrel_semantic_tree_binder::SemanticBinder;
//!
//! // Bind phase (takes a SemanticModel built elsewhere)
//! let model = SemanticBinder::bind(model, &mut diagnostics);
//! ```

// Core modules
mod debug;
pub mod resolution;
mod declaration_binder;
mod binders;
pub mod syntax;
mod maps;

// Feature modules
pub mod body_resolver;
pub mod diagnostics;

// Re-exports for convenient access
pub use debug::{
    format_type, print_model_symbols, print_semantic_model,
};
pub use kestrel_semantic_model::SemanticModel;
pub use resolution::{LocalScope, SemanticBinder, TypeResolver};
pub use declaration_binder::{BindingContext, DeclarationBinder, DeclarationBinderRegistry};
