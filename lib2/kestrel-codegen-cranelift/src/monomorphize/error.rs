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
    /// An instantiation arrived at the monomorphizer with a type-arg count
    /// that does not match the function's declared `type_params` arity.
    /// Always a dispatch bug in MIR lowering: some call site constructed a
    /// callee whose `type_args` length is wrong for the target function.
    TypeArgArityMismatch {
        function: String,
        expected: usize,
        got: usize,
    },
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
            },
            Self::TypeMismatch { expected, found } => {
                write!(f, "type mismatch: expected {expected}, found {found}")
            },
            Self::TypeArgArityMismatch {
                function,
                expected,
                got,
            } => write!(
                f,
                "dispatch bug: call to '{function}' has {got} type arg(s), function expects {expected}"
            ),
        }
    }
}

impl std::error::Error for MonomorphizeError {}
