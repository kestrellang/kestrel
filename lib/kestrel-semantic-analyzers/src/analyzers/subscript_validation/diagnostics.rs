//! Diagnostic types for subscript validation errors.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when a subscript has no parameters.
///
/// Subscripts must have at least one parameter to distinguish them from
/// computed properties.
pub struct SubscriptMissingParametersError {
    pub span: Span,
}

impl IntoDiagnostic for SubscriptMissingParametersError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("subscript must have at least one parameter")
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message("add at least one parameter")])
            .with_notes(vec![
                "Subscripts provide indexed access and require parameters.".to_string(),
                "Use a computed property instead if no parameters are needed.".to_string(),
            ])
    }
}

/// Error when a subscript outside of a protocol has no body.
///
/// Subscripts must have a body unless they are protocol requirements.
pub struct SubscriptMissingBodyError {
    pub span: Span,
}

impl IntoDiagnostic for SubscriptMissingBodyError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("subscript must have a body")
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message("add a body to this subscript")])
            .with_notes(vec![
                "Provide a body with { expr } or { get { } set { } } syntax.".to_string(),
            ])
    }
}
