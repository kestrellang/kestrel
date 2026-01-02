//! Error types for code generation.

use crate::monomorphize::MonomorphizeError;
use std::fmt;

/// Errors that can occur during code generation.
#[derive(Debug)]
pub enum CodegenError {
    /// Failed to create the Cranelift module.
    ModuleCreation(String),
    /// Failed to compile a function.
    FunctionCompilation { name: String, error: String },
    /// Failed to define a function in the module.
    FunctionDefinition { name: String, error: String },
    /// Failed to finish the module.
    ModuleFinish(String),
    /// Entry point (main) not found.
    NoEntryPoint,
    /// Invalid entry point signature.
    InvalidEntryPoint(String),
    /// Linker error.
    LinkerError(String),
    /// I/O error.
    IoError(String),
    /// Unsupported feature.
    Unsupported(String),
    /// Failed to create data section entry.
    DataSection(String),
    /// Monomorphization error.
    Monomorphization(MonomorphizeError),
    /// Multiple monomorphization errors.
    MonomorphizationErrors(Vec<MonomorphizeError>),
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodegenError::ModuleCreation(e) => {
                write!(f, "failed to create module: {}", e)
            }
            CodegenError::FunctionCompilation { name, error } => {
                write!(f, "failed to compile function '{}': {}", name, error)
            }
            CodegenError::FunctionDefinition { name, error } => {
                write!(f, "failed to define function '{}': {}", name, error)
            }
            CodegenError::ModuleFinish(e) => {
                write!(f, "failed to finish module: {}", e)
            }
            CodegenError::NoEntryPoint => {
                write!(f, "no entry point 'main' found")
            }
            CodegenError::InvalidEntryPoint(msg) => {
                write!(f, "invalid entry point: {}", msg)
            }
            CodegenError::LinkerError(e) => {
                write!(f, "linker error: {}", e)
            }
            CodegenError::IoError(e) => {
                write!(f, "I/O error: {}", e)
            }
            CodegenError::Unsupported(msg) => {
                write!(f, "unsupported: {}", msg)
            }
            CodegenError::DataSection(msg) => {
                write!(f, "data section error: {}", msg)
            }
            CodegenError::Monomorphization(e) => {
                write!(f, "monomorphization error: {}", e)
            }
            CodegenError::MonomorphizationErrors(errors) => {
                writeln!(f, "monomorphization errors:")?;
                for e in errors {
                    writeln!(f, "  - {}", e)?;
                }
                Ok(())
            }
        }
    }
}

impl From<MonomorphizeError> for CodegenError {
    fn from(e: MonomorphizeError) -> Self {
        CodegenError::Monomorphization(e)
    }
}

impl From<Vec<MonomorphizeError>> for CodegenError {
    fn from(errors: Vec<MonomorphizeError>) -> Self {
        CodegenError::MonomorphizationErrors(errors)
    }
}

impl std::error::Error for CodegenError {}
