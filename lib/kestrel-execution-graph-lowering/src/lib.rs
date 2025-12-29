//! Kestrel Execution Graph Lowering
//!
//! This crate transforms the semantic tree (typed AST) into the execution graph (MIR).
//! It handles the conversion of high-level constructs like expressions, statements,
//! and control flow into the flat, explicit MIR representation.
//!
//! # Example
//!
//! ```ignore
//! use kestrel_execution_graph_lowering::lower_module;
//!
//! let result = lower_module(&model, &module_symbol);
//! if result.diagnostics.iter().any(|d| d.severity == Severity::Error) {
//!     // Handle errors
//! }
//! let mir = result.mir;
//! ```

mod closure;
mod context;
mod error;
mod expr;
mod lowerer;
mod match_lowering;
mod name;
mod pattern;
mod stmt;
mod ty;

pub use context::LoweringContext;
pub use error::LoweringError;

use kestrel_execution_graph::MirContext;
use kestrel_reporting::Diagnostic;
use kestrel_semantic_model::SemanticModel;
use kestrel_semantic_tree::language::KestrelLanguage;
use semantic_tree::symbol::Symbol;
use std::sync::Arc;

/// Result of lowering a module to MIR.
pub struct LoweringResult {
    /// The generated MIR context.
    pub mir: MirContext,
    /// Diagnostics (errors, warnings) produced during lowering.
    pub diagnostics: Vec<Diagnostic<usize>>,
}

impl LoweringResult {
    /// Check if any errors occurred during lowering.
    pub fn has_errors(&self) -> bool {
        use kestrel_reporting::Severity;
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error || d.severity == Severity::Bug)
    }
}

/// Lower a module and all its contents to MIR.
///
/// This is the main entry point for the lowering pass. It traverses the semantic tree
/// starting from the given module symbol and generates MIR for all items within.
///
/// # Arguments
///
/// * `model` - The semantic model providing context and queries
/// * `module` - The module symbol to lower (typically a SourceFile or Module)
///
/// # Returns
///
/// A `LoweringResult` containing the generated MIR and any diagnostics.
pub fn lower_module(
    model: &SemanticModel,
    module: &Arc<dyn Symbol<KestrelLanguage>>,
) -> LoweringResult {
    let mut ctx = LoweringContext::new(model);

    // Lower all children of the module
    for child in module.metadata().children() {
        lowerer::lower_item(&mut ctx, &child);
    }

    ctx.finish()
}
