//! Assignment validation errors.
//!
//! Errors related to assigning to immutable targets.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when assigning to a `let` binding.
pub struct CannotAssignToLetError {
    /// Span of the assignment expression
    pub assignment_span: Span,
    /// Span of the target expression
    pub target_span: Span,
    /// Name of the binding being assigned to
    pub binding_name: String,
    /// Span where the `let` binding was declared
    pub binding_span: Span,
}

impl IntoDiagnostic for CannotAssignToLetError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "cannot assign to immutable binding '{}'",
                self.binding_name
            ))
            .with_labels(vec![
                Label::primary(self.target_span.file_id, self.target_span.range())
                    .with_message("cannot assign to immutable binding"),
                Label::secondary(self.binding_span.file_id, self.binding_span.range())
                    .with_message("binding declared as immutable here"),
            ])
            .with_notes(vec![
                "help: consider declaring as 'var' instead".to_string(),
            ])
    }
}

/// Error when assigning to an immutable field.
pub struct CannotAssignToImmutableFieldError {
    /// Span of the assignment expression
    pub assignment_span: Span,
    /// Span of the target expression
    pub target_span: Span,
    /// Name of the field being assigned to
    pub field_name: String,
    /// Span where the field was declared (if available)
    pub field_span: Option<Span>,
}

impl IntoDiagnostic for CannotAssignToImmutableFieldError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let mut labels = vec![
            Label::primary(self.target_span.file_id, self.target_span.range())
                .with_message("cannot assign to immutable field"),
        ];

        if let Some(ref field_span) = self.field_span {
            labels.push(
                Label::secondary(field_span.file_id, field_span.range())
                    .with_message("field declared as immutable here"),
            );
        }

        Diagnostic::error()
            .with_message(format!(
                "cannot assign to immutable field '{}'",
                self.field_name
            ))
            .with_labels(labels)
            .with_notes(vec![
                "help: declare the field as 'var' if mutation is needed".to_string(),
            ])
    }
}

/// Error when assigning to a temporary value.
pub struct CannotAssignToTemporaryError {
    /// Span of the assignment expression
    pub assignment_span: Span,
    /// Span of the target expression (the temporary)
    pub target_span: Span,
}

impl IntoDiagnostic for CannotAssignToTemporaryError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("cannot assign to temporary value")
            .with_labels(vec![
                Label::primary(self.target_span.file_id, self.target_span.range())
                    .with_message("temporary value"),
            ])
            .with_notes(vec![
                "assignment requires a mutable variable or field".to_string(),
            ])
    }
}

/// Error when assigning through an immutable binding.
pub struct CannotAssignThroughImmutableBindingError {
    /// Span of the assignment expression
    pub assignment_span: Span,
    /// Span of the target expression
    pub target_span: Span,
    /// Name of the immutable root binding
    pub binding_name: String,
    /// Span where the binding was declared
    pub binding_span: Span,
    /// The full field path (e.g., "x" for "point.x")
    pub field_path: String,
}

impl IntoDiagnostic for CannotAssignThroughImmutableBindingError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "cannot assign to field '{}' of immutable binding '{}'",
                self.field_path, self.binding_name
            ))
            .with_labels(vec![
                Label::primary(self.target_span.file_id, self.target_span.range())
                    .with_message("cannot assign through immutable binding"),
                Label::secondary(self.binding_span.file_id, self.binding_span.range())
                    .with_message("binding declared as immutable here"),
            ])
            .with_notes(vec![format!(
                "help: declare '{}' as 'var' to allow mutation",
                self.binding_name
            )])
    }
}
