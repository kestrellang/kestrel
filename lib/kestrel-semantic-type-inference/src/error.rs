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

    /// No matching overload for call (wrong labels or arity).
    NoMatchingOverload {
        /// The name of the function/case being called
        name: String,
        /// The type being called on (for methods/cases)
        receiver_ty: Ty,
        /// The provided argument labels (None = unlabeled)
        provided_labels: Vec<Option<String>>,
        /// The expected argument labels
        expected_labels: Vec<Option<String>>,
        /// Where the call occurred
        span: Span,
    },

    /// Cannot infer enum type for shorthand syntax.
    CannotInferEnumType {
        /// The member name being accessed
        member_name: String,
        /// Where the shorthand was used
        span: Span,
    },

    /// Unknown field in struct pattern.
    UnknownStructField {
        /// The struct name
        struct_name: String,
        /// The unknown field name
        field_name: String,
        /// Where the pattern is
        span: Span,
    },

    /// Missing fields in struct pattern (without rest pattern).
    MissingStructFields {
        /// The struct name
        struct_name: String,
        /// The missing field names
        missing_fields: Vec<String>,
        /// Where the pattern is
        span: Span,
    },

    /// Unknown enum case in pattern.
    UnknownEnumCase {
        /// The enum name
        enum_name: String,
        /// The unknown case name
        case_name: String,
        /// Where the pattern is
        span: Span,
    },

    /// Tuple pattern has wrong arity.
    TupleArityMismatch {
        /// The expected number of elements
        expected: usize,
        /// The found number of elements
        found: usize,
        /// Where the pattern is
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

    /// Create a no matching overload error.
    pub fn no_matching_overload(
        name: String,
        receiver_ty: Ty,
        provided_labels: Vec<Option<String>>,
        expected_labels: Vec<Option<String>>,
        span: Span,
    ) -> Self {
        InferenceError::NoMatchingOverload {
            name,
            receiver_ty,
            provided_labels,
            expected_labels,
            span,
        }
    }

    /// Create a cannot infer enum type error.
    pub fn cannot_infer_enum_type(member_name: String, span: Span) -> Self {
        InferenceError::CannotInferEnumType { member_name, span }
    }

    /// Create an unknown struct field error.
    pub fn unknown_struct_field(struct_name: String, field_name: String, span: Span) -> Self {
        InferenceError::UnknownStructField {
            struct_name,
            field_name,
            span,
        }
    }

    /// Create a missing struct fields error.
    pub fn missing_struct_fields(struct_name: String, missing_fields: Vec<String>, span: Span) -> Self {
        InferenceError::MissingStructFields {
            struct_name,
            missing_fields,
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
            InferenceError::NoMatchingOverload { span, .. } => Some(span),
            InferenceError::CannotInferEnumType { span, .. } => Some(span),
            InferenceError::UnknownStructField { span, .. } => Some(span),
            InferenceError::MissingStructFields { span, .. } => Some(span),
            InferenceError::UnknownEnumCase { span, .. } => Some(span),
            InferenceError::TupleArityMismatch { span, .. } => Some(span),
        }
    }
}

impl InferenceError {
    /// Create an unknown enum case error.
    pub fn unknown_enum_case(enum_name: String, case_name: String, span: Span) -> Self {
        InferenceError::UnknownEnumCase {
            enum_name,
            case_name,
            span,
        }
    }

    /// Create a tuple arity mismatch error.
    pub fn tuple_arity_mismatch(expected: usize, found: usize, span: Span) -> Self {
        InferenceError::TupleArityMismatch {
            expected,
            found,
            span,
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
            InferenceError::NoMatchingOverload {
                name,
                receiver_ty,
                provided_labels,
                expected_labels,
                ..
            } => {
                let provided: Vec<_> = provided_labels.iter().map(|l| l.as_deref().unwrap_or("_")).collect();
                let expected: Vec<_> = expected_labels.iter().map(|l| l.as_deref().unwrap_or("_")).collect();
                write!(
                    f,
                    "no matching overload for '{}' on {}: provided ({}) but expected ({})",
                    name, receiver_ty, provided.join(", "), expected.join(", ")
                )
            }
            InferenceError::CannotInferEnumType { member_name, .. } => {
                write!(
                    f,
                    "cannot infer enum type for shorthand '.{}'",
                    member_name
                )
            }
            InferenceError::UnknownStructField {
                struct_name,
                field_name,
                ..
            } => {
                write!(
                    f,
                    "struct `{}` has no field `{}`",
                    struct_name, field_name
                )
            }
            InferenceError::MissingStructFields {
                struct_name,
                missing_fields,
                ..
            } => {
                write!(
                    f,
                    "pattern does not mention fields {} of `{}`",
                    missing_fields.join(", "),
                    struct_name
                )
            }
            InferenceError::UnknownEnumCase {
                enum_name,
                case_name,
                ..
            } => {
                write!(
                    f,
                    "enum `{}` has no case `{}`",
                    enum_name, case_name
                )
            }
            InferenceError::TupleArityMismatch {
                expected,
                found,
                ..
            } => {
                write!(
                    f,
                    "tuple pattern arity mismatch: expected {} elements, found {}",
                    expected, found
                )
            }
        }
    }
}

impl std::error::Error for InferenceError {}
