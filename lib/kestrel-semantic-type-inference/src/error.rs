//! Type inference errors.
//!
//! These errors are produced when the solver cannot find a consistent
//! type assignment that satisfies all constraints.

use kestrel_semantic_tree::ty::{Ty, TyId};
use kestrel_span::Span;

/// An error that occurred during type inference.
#[derive(Debug, Clone)]
pub enum InferenceError {
    /// Two types could not be unified.
    ///
    /// This is the most common error - it means the program has a type mismatch.
    TypeMismatch {
        /// The expected type (from context)
        expected: Ty,
        /// The actual type found
        found: Ty,
        /// Where the mismatch was detected
        span: Span,
    },

    /// Occurs check failed - would create an infinite type.
    ///
    /// This happens when unifying a type variable with a type that contains
    /// that same variable, e.g., `T = List[T]` without recursion.
    OccursCheck {
        /// The type variable that would become infinite
        var: TyId,
        /// The type that contains it
        ty: Ty,
        /// Where the error was detected
        span: Span,
    },

    /// A type does not conform to a required protocol.
    ConformanceFailure {
        /// The type that should conform
        ty: Ty,
        /// The protocol it should conform to
        protocol_name: String,
        /// Where the conformance was required
        span: Span,
    },

    /// A member was not found on a type.
    MemberNotFound {
        /// The type that was accessed
        receiver: Ty,
        /// The member that wasn't found
        member: String,
        /// Where the access occurred
        span: Span,
    },

    /// An associated type could not be resolved.
    AssociatedTypeNotFound {
        /// The container type
        container: Ty,
        /// The associated type name
        assoc_name: String,
        /// Where the projection occurred
        span: Span,
    },

    /// Not all types could be fully resolved.
    ///
    /// This means some inference placeholders couldn't be determined
    /// from the available constraints.
    Ambiguous {
        /// The type IDs that remained unresolved
        unresolved: Vec<TyId>,
    },

    /// Internal solver error (shouldn't happen in normal use).
    Internal {
        /// Description of what went wrong
        message: String,
    },

    /// Closure has wrong number of parameters.
    ClosureArityMismatch {
        /// The closure's actual parameter count
        actual: usize,
        /// The expected parameter count
        expected: usize,
        /// Where the closure is defined
        span: Span,
    },

    /// Closure return type doesn't match expected.
    ClosureReturnTypeMismatch {
        /// The closure's actual return type
        actual: Ty,
        /// The expected return type
        expected: Ty,
        /// Where the closure is defined
        span: Span,
    },

    /// Closure parameter type doesn't match expected.
    ClosureParamTypeMismatch {
        /// The parameter index (0-based)
        index: usize,
        /// The closure's actual parameter type
        actual: Ty,
        /// The expected parameter type
        expected: Ty,
        /// Where the closure is defined
        span: Span,
    },

    /// `it` used with wrong arity context.
    ItUsedWithWrongArity {
        /// The expected arity from context
        expected_arity: usize,
        /// Where the closure is defined
        span: Span,
    },
}

impl InferenceError {
    /// Create a type mismatch error.
    pub fn type_mismatch(expected: Ty, found: Ty, span: Span) -> Self {
        InferenceError::TypeMismatch {
            expected,
            found,
            span,
        }
    }

    /// Create an occurs check error.
    pub fn occurs_check(var: TyId, ty: Ty, span: Span) -> Self {
        InferenceError::OccursCheck { var, ty, span }
    }

    /// Create a conformance failure error.
    pub fn conformance_failure(ty: Ty, protocol_name: String, span: Span) -> Self {
        InferenceError::ConformanceFailure {
            ty,
            protocol_name,
            span,
        }
    }

    /// Create a member not found error.
    pub fn member_not_found(receiver: Ty, member: String, span: Span) -> Self {
        InferenceError::MemberNotFound {
            receiver,
            member,
            span,
        }
    }

    /// Create an associated type not found error.
    pub fn associated_type_not_found(container: Ty, assoc_name: String, span: Span) -> Self {
        InferenceError::AssociatedTypeNotFound {
            container,
            assoc_name,
            span,
        }
    }

    /// Create an ambiguous error.
    pub fn ambiguous(unresolved: Vec<TyId>) -> Self {
        InferenceError::Ambiguous { unresolved }
    }

    /// Create an internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        InferenceError::Internal {
            message: message.into(),
        }
    }

    /// Create a closure arity mismatch error.
    pub fn closure_arity_mismatch(actual: usize, expected: usize, span: Span) -> Self {
        InferenceError::ClosureArityMismatch {
            actual,
            expected,
            span,
        }
    }

    /// Create a closure return type mismatch error.
    pub fn closure_return_type_mismatch(actual: Ty, expected: Ty, span: Span) -> Self {
        InferenceError::ClosureReturnTypeMismatch {
            actual,
            expected,
            span,
        }
    }

    /// Create a closure parameter type mismatch error.
    pub fn closure_param_type_mismatch(
        index: usize,
        actual: Ty,
        expected: Ty,
        span: Span,
    ) -> Self {
        InferenceError::ClosureParamTypeMismatch {
            index,
            actual,
            expected,
            span,
        }
    }

    /// Create an `it` used with wrong arity error.
    pub fn it_used_with_wrong_arity(expected_arity: usize, span: Span) -> Self {
        InferenceError::ItUsedWithWrongArity {
            expected_arity,
            span,
        }
    }

    /// Get the span associated with this error, if any.
    pub fn span(&self) -> Option<&Span> {
        match self {
            InferenceError::TypeMismatch { span, .. } => Some(span),
            InferenceError::OccursCheck { span, .. } => Some(span),
            InferenceError::ConformanceFailure { span, .. } => Some(span),
            InferenceError::MemberNotFound { span, .. } => Some(span),
            InferenceError::AssociatedTypeNotFound { span, .. } => Some(span),
            InferenceError::Ambiguous { .. } => None,
            InferenceError::Internal { .. } => None,
            InferenceError::ClosureArityMismatch { span, .. } => Some(span),
            InferenceError::ClosureReturnTypeMismatch { span, .. } => Some(span),
            InferenceError::ClosureParamTypeMismatch { span, .. } => Some(span),
            InferenceError::ItUsedWithWrongArity { span, .. } => Some(span),
        }
    }
}

impl std::fmt::Display for InferenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InferenceError::TypeMismatch {
                expected, found, ..
            } => {
                write!(f, "type mismatch: expected {}, found {}", expected, found)
            }
            InferenceError::OccursCheck { var, ty, .. } => {
                write!(f, "infinite type: {:?} = {}", var, ty)
            }
            InferenceError::ConformanceFailure {
                ty, protocol_name, ..
            } => {
                write!(f, "type {} does not conform to {}", ty, protocol_name)
            }
            InferenceError::MemberNotFound {
                receiver, member, ..
            } => {
                write!(f, "no member '{}' on type {}", member, receiver)
            }
            InferenceError::AssociatedTypeNotFound {
                container,
                assoc_name,
                ..
            } => {
                write!(
                    f,
                    "no associated type '{}' on type {}",
                    assoc_name, container
                )
            }
            InferenceError::Ambiguous { unresolved } => {
                write!(f, "ambiguous type: {} unresolved", unresolved.len())
            }
            InferenceError::Internal { message } => {
                write!(f, "internal error: {}", message)
            }
            InferenceError::ClosureArityMismatch {
                actual, expected, ..
            } => {
                write!(
                    f,
                    "closure has {} parameters but {} expected",
                    actual, expected
                )
            }
            InferenceError::ClosureReturnTypeMismatch {
                actual, expected, ..
            } => {
                write!(
                    f,
                    "closure returns `{}` but `{}` expected",
                    actual, expected
                )
            }
            InferenceError::ClosureParamTypeMismatch {
                index,
                actual,
                expected,
                ..
            } => {
                write!(
                    f,
                    "closure parameter {} has type `{}` but `{}` expected",
                    index + 1,
                    actual,
                    expected
                )
            }
            InferenceError::ItUsedWithWrongArity { expected_arity, .. } => {
                write!(
                    f,
                    "`it` can only be used when closure has exactly 1 parameter, but {} expected",
                    expected_arity
                )
            }
        }
    }
}

impl std::error::Error for InferenceError {}
