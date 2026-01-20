//! Error types for compilation.

use kestrel_codegen_cranelift::CodegenError;
use kestrel_reporting::Diagnostic;
use std::fmt;

/// Errors that can occur during compilation to native code.
#[derive(Debug)]
pub enum CompileError {
    /// Semantic analysis produced errors.
    SemanticErrors,
    /// No semantic model was produced.
    NoSemanticModel,
    /// MIR lowering failed.
    LoweringFailed(Vec<Diagnostic<usize>>),
    /// No main function found (required for executables).
    NoMainFunction,
    /// Code generation failed.
    CodegenFailed(CodegenError),
    /// I/O error.
    IoError(String),
    /// Execution failed.
    ExecutionFailed(String),
    /// Invalid target triple.
    InvalidTarget(String),
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::SemanticErrors => {
                write!(f, "compilation failed due to semantic errors")
            }
            CompileError::NoSemanticModel => {
                write!(f, "no semantic model was produced")
            }
            CompileError::LoweringFailed(diagnostics) => {
                writeln!(f, "lowering to execution graph failed:")?;
                for diag in diagnostics {
                    writeln!(f, "  - {}", diag.message)?;
                }
                Ok(())
            }
            CompileError::NoMainFunction => {
                write!(f, "no 'main' function found")
            }
            CompileError::CodegenFailed(e) => {
                write!(f, "code generation failed: {}", e)
            }
            CompileError::IoError(e) => {
                write!(f, "I/O error: {}", e)
            }
            CompileError::ExecutionFailed(e) => {
                write!(f, "execution failed: {}", e)
            }
            CompileError::InvalidTarget(e) => {
                write!(f, "invalid target: {}", e)
            }
        }
    }
}

impl std::error::Error for CompileError {}

impl From<CodegenError> for CompileError {
    fn from(e: CodegenError) -> Self {
        CompileError::CodegenFailed(e)
    }
}

impl From<std::io::Error> for CompileError {
    fn from(e: std::io::Error) -> Self {
        CompileError::IoError(e.to_string())
    }
}
