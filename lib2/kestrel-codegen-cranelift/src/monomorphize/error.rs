//! Monomorphization error types.

use std::fmt;

/// Errors that occur during monomorphization (instantiation discovery).
#[derive(Debug, Clone)]
pub enum MonomorphizeError {
    /// A protocol witness was not found for a type.
    WitnessNotFound {
        protocol_name: String,
        type_description: String,
    },
    /// A method was not found in a witness.
    MethodNotFound {
        protocol_name: String,
        method: String,
        type_description: String,
    },
    /// A function was not found.
    FunctionNotFound { name: String },
    /// A type mismatch during witness pattern matching.
    TypeMismatch { expected: String, found: String },
}

impl fmt::Display for MonomorphizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WitnessNotFound {
                protocol_name,
                type_description,
            } => write!(
                f,
                "no witness found: {type_description} does not implement {protocol_name}"
            ),
            Self::MethodNotFound {
                protocol_name,
                method,
                type_description,
            } => write!(
                f,
                "method '{method}' not found in witness for {type_description}: {protocol_name}"
            ),
            Self::FunctionNotFound { name } => {
                write!(f, "function not found: {name}")
            }
            Self::TypeMismatch { expected, found } => {
                write!(f, "type mismatch: expected {expected}, found {found}")
            }
        }
    }
}

impl std::error::Error for MonomorphizeError {}
