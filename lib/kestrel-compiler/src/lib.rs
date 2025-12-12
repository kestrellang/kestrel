//! # Kestrel Compiler
//!
//! This crate provides a high-level compilation API for the Kestrel language,
//! inspired by Roslyn's `Compilation` API.
//!
//! ## Example
//!
//! ```no_run
//! use kestrel_compiler::Compilation;
//!
//! // Create a compilation with multiple source files
//! let compilation = Compilation::builder()
//!     .add_source("main.ks", "module Main\nclass Foo {}")
//!     .add_source("utils.ks", "module Utils\nclass Helper {}")
//!     .build();
//!
//! // Check for errors and emit diagnostics
//! if compilation.has_errors() {
//!     compilation.diagnostics().emit().unwrap();
//!     std::process::exit(1);
//! }
//!
//! // Access compiled results
//! for file in compilation.source_files() {
//!     println!("Compiled: {}", file.name());
//! }
//!
//! // Access the semantic model
//! if let Some(model) = compilation.semantic_model() {
//!     println!("Root: {:?}", model.root().metadata().name());
//! }
//! ```

mod builder;
mod compilation;
mod source_file;

pub use builder::CompilationBuilder;
pub use compilation::Compilation;
pub use source_file::SourceFile;

// Re-export commonly used types from dependencies
pub use kestrel_reporting::{Diagnostic, DiagnosticContext, IntoDiagnostic, Label, Severity};
pub use kestrel_semantic_model::SemanticModel;
pub use kestrel_semantic_tree_binder::SemanticTree;
pub use kestrel_syntax_tree::SyntaxNode;
