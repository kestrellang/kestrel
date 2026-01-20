//! # Kestrel Compiler
//!
//! This crate provides a high-level compilation API for the Kestrel language,
//! inspired by Roslyn's `Compilation` API.
//!
//! ## Example
//!
//! ```no_run
//! use kestrel_compiler::{Compilation, TargetConfig};
//!
//! // Create a compilation with multiple source files
//! let compilation = Compilation::builder()
//!     .add_source("main.ks", "module Main\nfunc main() {}")
//!     .build();
//!
//! // Check for errors and emit diagnostics
//! if compilation.has_errors() {
//!     compilation.diagnostics().emit().unwrap();
//!     std::process::exit(1);
//! }
//!
//! // Build an executable
//! let target = TargetConfig::host();
//! compilation.build(&target, &Default::default(), "output".as_ref()).unwrap();
//!
//! // Or run the program directly
//! let result = compilation.run(&target).unwrap();
//! println!("Exit code: {}", result.exit_code);
//! ```

mod builder;
mod compilation;
pub mod error;
pub mod run;
mod source_file;
pub mod stdlib;

pub use builder::CompilationBuilder;
pub use compilation::Compilation;
pub use error::CompileError;
pub use run::RunResult;
pub use source_file::SourceFile;
pub use stdlib::{StdLib, StdLibConfig, StdLibError};

// Re-export commonly used types from dependencies
pub use kestrel_reporting::{Diagnostic, DiagnosticContext, IntoDiagnostic, Label, Severity};
pub use kestrel_semantic_model::SemanticModel;
pub use kestrel_syntax_tree::SyntaxNode;

// Re-export codegen types
pub use kestrel_codegen::TargetConfig;
pub use kestrel_codegen_cranelift::CodegenOptions;
pub use kestrel_execution_graph::MirContext;
pub use kestrel_execution_graph_lowering::LoweringResult;
