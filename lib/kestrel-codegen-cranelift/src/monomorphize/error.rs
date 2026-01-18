//! Error types for monomorphization.

use kestrel_execution_graph::{Id, QualifiedName, Ty};
use std::fmt;

/// Errors that can occur during monomorphization.
#[derive(Debug, Clone)]
pub enum MonomorphizeError {
    /// No witness found for a protocol implementation.
    WitnessNotFound {
        protocol: Id<QualifiedName>,
        for_type: Id<Ty>,
    },

    /// A method was not found in a witness.
    MethodNotFoundInWitness {
        protocol: Id<QualifiedName>,
        method: String,
        for_type: Id<Ty>,
    },

    /// A function was not found by name.
    FunctionNotFound { name: Id<QualifiedName> },

    /// A function reference could not be instantiated.
    UnsupportedFunctionReference { name: Id<QualifiedName>, reason: String },

    /// Type mismatch during pattern matching.
    TypeMismatch { expected: Id<Ty>, found: Id<Ty> },

    /// A type was not interned during the collection phase.
    ///
    /// This indicates a bug in the collection algorithm - all types
    /// needed during codegen should have been interned during collection.
    TypeNotInterned { description: String },
}

impl fmt::Display for MonomorphizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MonomorphizeError::WitnessNotFound { protocol, for_type } => {
                write!(
                    f,
                    "no witness found: protocol {:?} for type {:?}",
                    protocol, for_type
                )
            }
            MonomorphizeError::MethodNotFoundInWitness {
                protocol,
                method,
                for_type,
            } => {
                write!(
                    f,
                    "method '{}' not found in witness: protocol {:?} for type {:?}",
                    method, protocol, for_type
                )
            }
            MonomorphizeError::FunctionNotFound { name } => {
                write!(f, "function not found: {:?}", name)
            }
            MonomorphizeError::UnsupportedFunctionReference { name, reason } => {
                write!(f, "unsupported function reference: {:?} ({})", name, reason)
            }
            MonomorphizeError::TypeMismatch { expected, found } => {
                write!(
                    f,
                    "type mismatch: expected {:?}, found {:?}",
                    expected, found
                )
            }
            MonomorphizeError::TypeNotInterned { description } => {
                write!(
                    f,
                    "type not interned (bug in collection phase): {}",
                    description
                )
            }
        }
    }
}

impl std::error::Error for MonomorphizeError {}
