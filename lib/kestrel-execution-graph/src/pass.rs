//! MIR pass system.

use crate::MirContext;
use crate::id::{Function, Id};
use kestrel_reporting::{Diagnostic, Severity};

/// Result of running a pass.
#[derive(Debug, Default)]
pub struct PassResult {
    /// Diagnostics (errors, warnings) produced by the pass.
    pub diagnostics: Vec<Diagnostic<usize>>,

    /// Whether the pass made any modifications.
    pub modified: bool,
}

impl PassResult {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a diagnostic.
    pub fn add_diagnostic(&mut self, diag: Diagnostic<usize>) {
        self.diagnostics.push(diag);
    }

    /// Create a result with a single diagnostic.
    pub fn with_diagnostic(mut self, diag: Diagnostic<usize>) -> Self {
        self.diagnostics.push(diag);
        self
    }

    /// Mark that the pass made modifications.
    pub fn set_modified(&mut self) {
        self.modified = true;
    }

    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error || d.severity == Severity::Bug)
    }

    /// Merge another result into this one.
    pub fn merge(&mut self, other: PassResult) {
        self.diagnostics.extend(other.diagnostics);
        self.modified |= other.modified;
    }
}

/// A transformation pass over the entire MIR.
pub trait MirPass {
    /// Human-readable name (used in Prior records).
    fn name(&self) -> &'static str;

    /// Run the pass, potentially modifying the context.
    fn run(&mut self, ctx: &mut MirContext) -> PassResult;
}

/// A pass that operates on individual functions.
pub trait FunctionPass {
    /// Human-readable name.
    fn name(&self) -> &'static str;

    /// Run the pass on a single function.
    fn run_on_function(&mut self, ctx: &mut MirContext, func: Id<Function>) -> PassResult;
}

/// Adapter to run a FunctionPass as a MirPass.
pub struct FunctionPassAdapter<P> {
    pass: P,
}

impl<P: FunctionPass> FunctionPassAdapter<P> {
    pub fn new(pass: P) -> Self {
        Self { pass }
    }
}

impl<P: FunctionPass> MirPass for FunctionPassAdapter<P> {
    fn name(&self) -> &'static str {
        self.pass.name()
    }

    fn run(&mut self, ctx: &mut MirContext) -> PassResult {
        let mut result = PassResult::new();

        // Collect function IDs to avoid borrowing issues
        let func_ids: Vec<_> = ctx.functions.ids().collect();

        for func_id in func_ids {
            let func_result = self.pass.run_on_function(ctx, func_id);
            result.merge(func_result);
        }

        result
    }
}

/// A pass manager that runs multiple passes in sequence.
pub struct PassManager {
    passes: Vec<Box<dyn MirPass>>,
}

impl PassManager {
    pub fn new() -> Self {
        Self { passes: Vec::new() }
    }

    /// Add a pass to the manager.
    pub fn add_pass<P: MirPass + 'static>(&mut self, pass: P) {
        self.passes.push(Box::new(pass));
    }

    /// Add a function pass to the manager.
    pub fn add_function_pass<P: FunctionPass + 'static>(&mut self, pass: P) {
        self.passes.push(Box::new(FunctionPassAdapter::new(pass)));
    }

    /// Run all passes in order.
    pub fn run(&mut self, ctx: &mut MirContext) -> PassResult {
        let mut result = PassResult::new();

        for pass in &mut self.passes {
            let pass_result = pass.run(ctx);
            result.merge(pass_result);

            // Stop early on errors
            if result.has_errors() {
                break;
            }
        }

        result
    }

    /// Run passes until no more modifications are made (fixed point).
    pub fn run_to_fixpoint(&mut self, ctx: &mut MirContext, max_iterations: usize) -> PassResult {
        let mut result = PassResult::new();

        for _ in 0..max_iterations {
            let mut iteration_result = PassResult::new();

            for pass in &mut self.passes {
                let pass_result = pass.run(ctx);
                iteration_result.merge(pass_result);

                if iteration_result.has_errors() {
                    result.merge(iteration_result);
                    return result;
                }
            }

            result.merge(PassResult {
                diagnostics: iteration_result.diagnostics,
                modified: false, // We'll set this below if needed
            });

            if !iteration_result.modified {
                break;
            }

            result.modified = true;
        }

        result
    }
}

impl Default for PassManager {
    fn default() -> Self {
        Self::new()
    }
}
