//! Member access errors.
//!
//! Errors related to accessing members (fields, methods, etc.) on types.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when a member cannot be found on a type.
pub struct NoSuchMemberError {
    /// Span of the member name being accessed
    pub member_span: Span,
    /// Name of the member being accessed
    pub member_name: String,
    /// Span of the base expression
    pub base_span: Span,
    /// String representation of the base type
    pub base_type: String,
}

impl IntoDiagnostic for NoSuchMemberError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "no member '{}' on type '{}'",
                self.member_name, self.base_type
            ))
            .with_labels(vec![
                Label::primary(self.member_span.file_id, self.member_span.range())
                    .with_message("unknown member"),
                Label::secondary(self.base_span.file_id, self.base_span.range())
                    .with_message(format!("has type '{}'", self.base_type)),
            ])
    }
}

/// Error when a member exists but is not visible from the current scope.
pub struct MemberNotVisibleError {
    /// Span of the member name being accessed
    pub member_span: Span,
    /// Name of the member being accessed
    pub member_name: String,
    /// Span of the base expression
    pub base_span: Span,
    /// String representation of the base type
    pub base_type: String,
    /// The visibility of the member
    pub visibility: String,
}

impl IntoDiagnostic for MemberNotVisibleError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "member '{}' is {} and not accessible from this scope",
                self.member_name, self.visibility
            ))
            .with_labels(vec![
                Label::primary(self.member_span.file_id, self.member_span.range())
                    .with_message(format!("{} member", self.visibility)),
                Label::secondary(self.base_span.file_id, self.base_span.range())
                    .with_message(format!("has type '{}'", self.base_type)),
            ])
    }
}

/// Error when a member exists but is not accessible as a value.
pub struct MemberNotAccessibleError {
    /// Span of the member name being accessed
    pub member_span: Span,
    /// Name of the member being accessed
    pub member_name: String,
    /// Span of the base expression
    pub base_span: Span,
    /// String representation of the base type
    pub base_type: String,
    /// What kind of member it is (e.g., "type alias", "associated type")
    pub member_kind: String,
}

impl IntoDiagnostic for MemberNotAccessibleError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "'{}' is a {} and cannot be used as a value",
                self.member_name, self.member_kind
            ))
            .with_labels(vec![
                Label::primary(self.member_span.file_id, self.member_span.range())
                    .with_message(format!("is a {}", self.member_kind)),
                Label::secondary(self.base_span.file_id, self.base_span.range())
                    .with_message(format!("has type '{}'", self.base_type)),
            ])
    }
}

/// Error when trying to access a member on a type that doesn't support member access.
pub struct CannotAccessMemberOnTypeError {
    /// Span of the entire access expression
    pub span: Span,
    /// String representation of the base type
    pub base_type: String,
}

impl IntoDiagnostic for CannotAccessMemberOnTypeError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("cannot access member on type '{}'", self.base_type))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("member access not supported"),
            ])
            .with_notes(vec![format!(
                "type '{}' does not have accessible members",
                self.base_type
            )])
    }
}

/// Error when tuple index is out of bounds.
pub struct TupleIndexOutOfBoundsError {
    /// Span of the index
    pub index_span: Span,
    /// The index that was accessed
    pub index: usize,
    /// The number of elements in the tuple
    pub tuple_length: usize,
    /// String representation of the tuple type
    pub tuple_type: String,
}

impl IntoDiagnostic for TupleIndexOutOfBoundsError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "tuple index {} is out of bounds for tuple of length {}",
                self.index, self.tuple_length
            ))
            .with_labels(vec![
                Label::primary(self.index_span.file_id, self.index_span.range())
                    .with_message(format!("index {} out of bounds", self.index)),
            ])
            .with_notes(vec![format!(
                "type '{}' has {} element{}",
                self.tuple_type,
                self.tuple_length,
                if self.tuple_length == 1 { "" } else { "s" }
            )])
    }
}

/// Error when trying to use tuple indexing on a non-tuple type.
pub struct TupleIndexOnNonTupleError {
    /// Span of the entire expression
    pub span: Span,
    /// The index that was accessed
    pub index: usize,
    /// String representation of the base type
    pub base_type: String,
}

impl IntoDiagnostic for TupleIndexOnNonTupleError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "cannot use tuple index on type '{}'",
                self.base_type
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("not a tuple type"),
            ])
            .with_notes(vec![format!(
                "tuple indexing (e.g., '.{}') can only be used on tuple types",
                self.index
            )])
    }
}

// =============================================================================
// Constraint Enforcement Errors
// =============================================================================

/// Error when accessing a member on a type parameter with no protocol bounds.
pub struct UnconstrainedTypeParameterMemberError {
    /// Span of the member access expression
    pub span: Span,
    /// Name of the member being accessed
    pub member_name: String,
    /// Name of the type parameter
    pub type_param_name: String,
}

impl IntoDiagnostic for UnconstrainedTypeParameterMemberError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "cannot call '{}' on type '{}'",
                self.member_name, self.type_param_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range()).with_message(format!(
                    "cannot call '{}' on type '{}'",
                    self.member_name, self.type_param_name
                )),
            ])
            .with_notes(vec![
                format!(
                    "'{}' is a type parameter with no constraints",
                    self.type_param_name
                ),
                format!(
                    "help: add a constraint: `where {}: SomeProtocol`",
                    self.type_param_name
                ),
            ])
    }
}

/// Error when a method is not found in any of the type parameter's protocol bounds.
pub struct MethodNotInBoundsError {
    /// Span of the method call
    pub call_span: Span,
    /// Name of the method being called
    pub method_name: String,
    /// Name of the type parameter
    pub type_param_name: String,
    /// Names of the protocol bounds
    pub bound_names: Vec<String>,
}

impl IntoDiagnostic for MethodNotInBoundsError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let bounds_str = if self.bound_names.is_empty() {
            "no protocol bounds".to_string()
        } else {
            self.bound_names.join(", ")
        };

        Diagnostic::error()
            .with_message(format!(
                "no method '{}' found for type '{}'",
                self.method_name, self.type_param_name
            ))
            .with_labels(vec![
                Label::primary(self.call_span.file_id, self.call_span.range())
                    .with_message("method not found"),
            ])
            .with_notes(vec![
                format!(
                    "'{}' is constrained to: {}",
                    self.type_param_name, bounds_str
                ),
                format!(
                    "none of these protocols have a method named '{}'",
                    self.method_name
                ),
            ])
    }
}

/// Error when a method is found in multiple protocol bounds with the same signature.
pub struct AmbiguousConstrainedMethodError {
    /// Span of the method call
    pub call_span: Span,
    /// Name of the method being called
    pub method_name: String,
    /// Names of the protocols that have this method
    pub protocol_names: Vec<String>,
    /// Spans of the method definitions in each protocol
    pub definition_spans: Vec<(String, Span)>,
}

impl IntoDiagnostic for AmbiguousConstrainedMethodError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let mut labels = vec![
            Label::primary(self.call_span.file_id, self.call_span.range())
                .with_message("ambiguous method call"),
        ];

        // Add secondary labels for each definition
        for (proto_name, span) in &self.definition_spans {
            labels.push(
                Label::secondary(span.file_id, span.range())
                    .with_message(format!("candidate from '{}'", proto_name)),
            );
        }

        Diagnostic::error()
            .with_message(format!(
                "ambiguous method call '{}': found in multiple protocols",
                self.method_name
            ))
            .with_labels(labels)
            .with_notes(vec![format!(
                "'{}' is defined in: {}",
                self.method_name,
                self.protocol_names.join(", ")
            )])
    }
}

/// Error when a type argument does not satisfy a constraint.
pub struct ConstraintNotSatisfiedError {
    /// Span of the call site
    pub call_span: Span,
    /// The type that doesn't satisfy the constraint
    pub type_name: String,
    /// The protocol constraint that's not satisfied
    pub constraint_name: String,
    /// Name of the type parameter
    pub type_param_name: String,
    /// Span of the constraint declaration (in the function signature)
    pub constraint_span: Option<Span>,
}

impl IntoDiagnostic for ConstraintNotSatisfiedError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let mut labels = vec![
            Label::primary(self.call_span.file_id, self.call_span.range())
                .with_message("constraint not satisfied"),
        ];

        if let Some(ref span) = self.constraint_span {
            labels.push(
                Label::secondary(span.file_id, span.range()).with_message(format!(
                    "required by this constraint on '{}'",
                    self.type_param_name
                )),
            );
        }

        Diagnostic::error()
            .with_message(format!(
                "type '{}' does not satisfy constraint '{}'",
                self.type_name, self.constraint_name
            ))
            .with_labels(labels)
            .with_notes(vec![format!(
                "'{}' does not conform to '{}'",
                self.type_name, self.constraint_name
            )])
    }
}

/// Diagnostic (warning/note) when a generic protocol bound is used.
/// These require associated types which aren't implemented yet.
pub struct UnsupportedGenericProtocolBoundError {
    /// Span of the protocol bound
    pub span: Span,
    /// Name of the protocol
    pub protocol_name: String,
}

impl IntoDiagnostic for UnsupportedGenericProtocolBoundError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "generic protocol bounds are not yet supported: '{}'",
                self.protocol_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("generic protocol bound"),
            ])
            .with_notes(vec![
                "generic protocol bounds require associated types".to_string(),
                "this feature is coming in a future version".to_string(),
            ])
    }
}

// =============================================================================
// Type Parameter Init/Static Method Errors
// =============================================================================

/// Error when calling init on a type parameter with no init in its bounds.
pub struct NoInitInTypeParameterBoundsError {
    /// Span of the init call expression
    pub span: Span,
    /// Name of the type parameter
    pub type_param_name: String,
    /// Names of the protocol bounds
    pub bound_names: Vec<String>,
}

impl IntoDiagnostic for NoInitInTypeParameterBoundsError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let bounds_str = if self.bound_names.is_empty() {
            "no protocol bounds".to_string()
        } else {
            self.bound_names.join(", ")
        };

        Diagnostic::error()
            .with_message(format!(
                "no initializer found for type parameter '{}'",
                self.type_param_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("no matching initializer"),
            ])
            .with_notes(vec![
                format!(
                    "'{}' is constrained to: {}",
                    self.type_param_name, bounds_str
                ),
                "none of these protocols have an initializer".to_string(),
            ])
    }
}

/// Error when calling init on a type parameter but no matching signature is found.
pub struct NoMatchingTypeParameterInitError {
    /// Span of the init call expression
    pub span: Span,
    /// Name of the type parameter
    pub type_param_name: String,
    /// The argument labels provided
    pub provided_labels: Vec<Option<String>>,
    /// Number of arguments provided
    pub provided_arity: usize,
    /// Available init overloads from protocol bounds
    pub available_inits: Vec<super::call::OverloadDescription>,
}

impl IntoDiagnostic for NoMatchingTypeParameterInitError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let provided = super::call::format_argument_labels(&self.provided_labels);

        let mut labels = vec![
            Label::primary(self.span.file_id, self.span.range()).with_message(format!(
                "no matching initializer for {} argument(s) with labels {}",
                self.provided_arity, provided
            )),
        ];

        // Add secondary labels for available initializers
        for init in &self.available_inits {
            if let (Some(span), Some(def_file_id)) =
                (&init.definition_span, init.definition_file_id)
            {
                labels.push(
                    Label::secondary(def_file_id, span.range())
                        .with_message(format!("candidate: {}", init.display())),
                );
            }
        }

        let mut notes = vec![format!(
            "type parameter '{}' has these available initializers:",
            self.type_param_name
        )];
        for init in &self.available_inits {
            notes.push(format!("  - {}", init.display()));
        }

        Diagnostic::error()
            .with_message(format!(
                "no matching initializer for type parameter '{}'",
                self.type_param_name
            ))
            .with_labels(labels)
            .with_notes(notes)
    }
}

/// Error when multiple protocols have matching init with same signature.
pub struct AmbiguousTypeParameterInitError {
    /// Span of the init call expression
    pub span: Span,
    /// Name of the type parameter
    pub type_param_name: String,
    /// Names of the protocols that have matching init
    pub protocol_names: Vec<String>,
    /// Spans of the init definitions in each protocol
    pub definition_spans: Vec<(String, Span)>,
}

impl IntoDiagnostic for AmbiguousTypeParameterInitError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let mut labels = vec![
            Label::primary(self.span.file_id, self.span.range())
                .with_message("ambiguous initializer call"),
        ];

        // Add secondary labels for each definition
        for (proto_name, span) in &self.definition_spans {
            labels.push(
                Label::secondary(span.file_id, span.range())
                    .with_message(format!("candidate from '{}'", proto_name)),
            );
        }

        Diagnostic::error()
            .with_message(format!(
                "ambiguous initializer call on '{}': found in multiple protocols",
                self.type_param_name
            ))
            .with_labels(labels)
            .with_notes(vec![format!(
                "initializer is defined in: {}",
                self.protocol_names.join(", ")
            )])
    }
}

/// Error when a type parameter is used as a value without calling init or static method.
pub struct TypeParameterCannotBeUsedAsValueError {
    /// Span of the type parameter reference
    pub span: Span,
    /// Name of the type parameter
    pub type_param_name: String,
}

impl IntoDiagnostic for TypeParameterCannotBeUsedAsValueError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "type parameter '{}' cannot be used as a value",
                self.type_param_name
            ))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message("not a value")])
            .with_notes(vec![
                format!("'{}' is a type parameter, not a value", self.type_param_name),
                format!("hint: use '{}()' to call an initializer or '{}.staticMethod()' to call a static method",
                    self.type_param_name, self.type_param_name),
            ])
    }
}
