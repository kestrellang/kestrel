//! Error types for code generation.

use std::fmt;

/// Errors that can occur during code generation.
#[derive(Debug)]
pub enum CodegenError {
    /// Failed to create the Cranelift module.
    ModuleCreation(String),
    /// Failed to compile a function body.
    FunctionCompilation {
        name: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// Failed to define a compiled function in the module.
    FunctionDefinition {
        name: String,
        source: cranelift_module::ModuleError,
    },
    /// Failed to finish the object module.
    ModuleFinish(String),
    /// No entry point (main function) found.
    NoEntryPoint,
    /// The entry point is invalid (e.g., wrong signature).
    InvalidEntryPoint(String),
    /// Linker invocation failed.
    LinkerError(String),
    /// I/O error (e.g., writing object file).
    IoError(std::io::Error),
    /// Feature not yet supported.
    Unsupported(String),
    /// Error in the data section (statics, string literals).
    DataSection(String),
    /// Error during monomorphization.
    Monomorphization(String),
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModuleCreation(msg) => write!(f, "module creation failed: {msg}"),
            Self::FunctionCompilation { name, source } => {
                write!(f, "failed to compile function '{name}': {source}")
            },
            Self::FunctionDefinition { name, source } => {
                write!(f, "failed to define function '{name}': {source}")
            },
            Self::ModuleFinish(msg) => write!(f, "module finish failed: {msg}"),
            Self::NoEntryPoint => write!(f, "no entry point (main function) found"),
            Self::InvalidEntryPoint(msg) => write!(f, "invalid entry point: {msg}"),
            Self::LinkerError(msg) => write!(f, "linker error: {msg}"),
            Self::IoError(err) => write!(f, "I/O error: {err}"),
            Self::Unsupported(msg) => write!(f, "unsupported: {msg}"),
            Self::DataSection(msg) => write!(f, "data section error: {msg}"),
            Self::Monomorphization(msg) => write!(f, "monomorphization error: {msg}"),
        }
    }
}

impl std::error::Error for CodegenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::FunctionCompilation { source, .. } => Some(source.as_ref()),
            Self::FunctionDefinition { source, .. } => Some(source),
            Self::IoError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for CodegenError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}
