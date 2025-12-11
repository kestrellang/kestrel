//! Type checking errors.
//!
//! Errors related to type mismatches across the language.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when a value's type doesn't match the expected type.
///
/// This is a general type mismatch error used for:
/// - Return type mismatches
/// - Assignment type mismatches
/// - Variable binding type mismatches
/// - Function argument type mismatches
/// - Array element type mismatches
pub struct TypeMismatchError {
    /// The span where the type mismatch occurred
    pub span: Span,
    /// Human-readable description of the expected type
    pub expected: String,
    /// Human-readable description of the actual type found
    pub found: String,
    /// Context describing where the mismatch occurred (e.g., "return type", "argument 1")
    pub context: String,
}

impl IntoDiagnostic for TypeMismatchError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "type mismatch: expected `{}`, found `{}`",
                self.expected, self.found
            ))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message(format!("expected `{}`", self.expected))])
            .with_notes(vec![format!(
                "{}: expected `{}`, found `{}`",
                self.context, self.expected, self.found
            )])
    }
}

/// Error when a condition expression is not a Bool.
///
/// Used for if conditions and while conditions.
pub struct ConditionNotBoolError {
    /// The span of the condition expression
    pub span: Span,
    /// Human-readable description of the actual type found
    pub found: String,
    /// The kind of condition (e.g., "if", "while")
    pub condition_kind: &'static str,
}

impl IntoDiagnostic for ConditionNotBoolError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "{} condition must be `Bool`, found `{}`",
                self.condition_kind, self.found
            ))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message(format!("expected `Bool`, found `{}`", self.found))])
    }
}

/// Error when if/else branches have incompatible types.
pub struct BranchTypeMismatchError {
    /// The span of the if expression
    pub if_span: Span,
    /// The span of the then branch value
    pub then_span: Span,
    /// The span of the else branch value
    pub else_span: Span,
    /// Human-readable description of the then branch type
    pub then_type: String,
    /// Human-readable description of the else branch type
    pub else_type: String,
}

impl IntoDiagnostic for BranchTypeMismatchError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "if/else branches have incompatible types: `{}` vs `{}`",
                self.then_type, self.else_type
            ))
            .with_labels(vec![
                Label::primary(self.then_span.file_id, self.then_span.range())
                    .with_message(format!("this has type `{}`", self.then_type)),
                Label::secondary(self.else_span.file_id, self.else_span.range())
                    .with_message(format!("this has type `{}`", self.else_type)),
            ])
            .with_notes(vec![
                "if/else branches must have the same type when used as an expression".to_string(),
            ])
    }
}

/// Error when array elements have inconsistent types.
pub struct ArrayElementTypeMismatchError {
    /// The span of the array literal
    pub array_span: Span,
    /// The span of the first element (which determines expected type)
    pub first_element_span: Span,
    /// The span of the mismatched element
    pub element_span: Span,
    /// The index of the mismatched element
    pub element_index: usize,
    /// Human-readable description of the expected type (from first element)
    pub expected: String,
    /// Human-readable description of the actual type
    pub found: String,
}

impl IntoDiagnostic for ArrayElementTypeMismatchError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "array element type mismatch: expected `{}`, found `{}`",
                self.expected, self.found
            ))
            .with_labels(vec![
                Label::primary(self.element_span.file_id, self.element_span.range())
                    .with_message(format!("expected `{}`, found `{}`", self.expected, self.found)),
                Label::secondary(self.first_element_span.file_id, self.first_element_span.range())
                    .with_message(format!("first element has type `{}`", self.expected)),
            ])
            .with_notes(vec![format!(
                "element at index {} has type `{}`, but array elements must all be `{}`",
                self.element_index, self.found, self.expected
            )])
    }
}
