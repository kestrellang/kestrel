//! Access mode validation errors.
//!
//! Errors related to parameter access modes (borrow, mutating, consuming)
//! at call sites.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when passing a `let` binding to a `mutating` parameter.
pub struct CannotPassLetToMutatingError {
    /// Span of the entire call expression
    pub call_span: Span,
    /// Span of the argument expression
    pub argument_span: Span,
    /// Name of the binding being passed
    pub binding_name: String,
    /// Span where the `let` binding was declared
    pub binding_span: Span,
    /// Name of the parameter being passed to
    pub parameter_name: String,
}

impl IntoDiagnostic for CannotPassLetToMutatingError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "cannot pass 'let' binding '{}' to 'mutating' parameter",
                self.binding_name
            ))
            .with_labels(vec![
                Label::primary(self.argument_span.file_id, self.argument_span.range())
                    .with_message(format!(
                        "cannot pass to 'mutating' parameter '{}'",
                        self.parameter_name
                    )),
                Label::secondary(self.binding_span.file_id, self.binding_span.range())
                    .with_message("binding declared as 'let' here"),
            ])
            .with_notes(vec![
                "help: consider declaring as 'var' instead".to_string(),
            ])
    }
}

/// Error when passing an immutable field to a `mutating` parameter.
pub struct CannotPassImmutableFieldToMutatingError {
    /// Span of the entire call expression
    pub call_span: Span,
    /// Span of the argument expression
    pub argument_span: Span,
    /// Name of the field being accessed
    pub field_name: String,
    /// Span where the field was declared (if available)
    pub field_span: Option<Span>,
    /// Name of the parameter being passed to
    pub parameter_name: String,
}

impl IntoDiagnostic for CannotPassImmutableFieldToMutatingError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let mut labels = vec![
            Label::primary(self.argument_span.file_id, self.argument_span.range()).with_message(
                format!(
                    "cannot pass to 'mutating' parameter '{}'",
                    self.parameter_name
                ),
            ),
        ];

        if let Some(ref field_span) = self.field_span {
            labels.push(
                Label::secondary(field_span.file_id, field_span.range())
                    .with_message("field declared as 'let' here"),
            );
        }

        Diagnostic::error()
            .with_message(format!(
                "cannot pass immutable field '{}' to 'mutating' parameter",
                self.field_name
            ))
            .with_labels(labels)
            .with_notes(vec![
                "help: declare the field as 'var' if mutation is needed".to_string(),
            ])
    }
}

/// Error when passing a temporary value to a `mutating` parameter.
pub struct CannotPassTemporaryToMutatingError {
    /// Span of the entire call expression
    pub call_span: Span,
    /// Span of the argument expression (the temporary)
    pub argument_span: Span,
    /// Name of the parameter being passed to
    pub parameter_name: String,
}

impl IntoDiagnostic for CannotPassTemporaryToMutatingError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("cannot pass temporary value to 'mutating' parameter")
            .with_labels(vec![
                Label::primary(self.argument_span.file_id, self.argument_span.range())
                    .with_message("temporary value"),
            ])
            .with_notes(vec![
                "'mutating' parameters require a mutable variable or field".to_string(),
                "help: assign to a variable first".to_string(),
            ])
    }
}

/// Error when trying to mutate through an immutable binding.
pub struct CannotMutateThroughImmutableBindingError {
    /// Span of the entire call expression
    pub call_span: Span,
    /// Span of the argument expression
    pub argument_span: Span,
    /// Name of the immutable root binding
    pub binding_name: String,
    /// Span where the binding was declared
    pub binding_span: Span,
    /// The full field path (e.g., "shape.origin")
    pub field_path: String,
    /// Name of the parameter being passed to
    pub parameter_name: String,
}

impl IntoDiagnostic for CannotMutateThroughImmutableBindingError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "cannot pass field '{}' of immutable binding '{}' to 'mutating' parameter",
                self.field_path, self.binding_name
            ))
            .with_labels(vec![
                Label::primary(self.argument_span.file_id, self.argument_span.range())
                    .with_message(format!(
                        "cannot pass to 'mutating' parameter '{}'",
                        self.parameter_name
                    )),
                Label::secondary(self.binding_span.file_id, self.binding_span.range())
                    .with_message("binding declared as 'let' here"),
            ])
            .with_notes(vec![format!(
                "help: declare '{}' as 'var' to allow mutation",
                self.binding_name
            )])
    }
}
