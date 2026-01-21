//! Control flow errors.
//!
//! Errors related to break, continue, and loop constructs.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when `break` is used outside of a loop
pub struct BreakOutsideLoopError {
    pub span: Span,
}

impl IntoDiagnostic for BreakOutsideLoopError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("`break` outside of loop")
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("`break` can only be used inside a loop"),
            ])
    }
}

/// Error when `continue` is used outside of a loop
pub struct ContinueOutsideLoopError {
    pub span: Span,
}

impl IntoDiagnostic for ContinueOutsideLoopError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("`continue` outside of loop")
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("`continue` can only be used inside a loop"),
            ])
    }
}

/// Error when a label is used that doesn't exist
pub struct UndeclaredLabelError {
    pub span: Span,
    pub label_name: String,
}

impl IntoDiagnostic for UndeclaredLabelError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("use of undeclared label `{}`", self.label_name))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("undeclared label"),
            ])
            .with_notes(vec![
                "labels must be declared on a loop using `label: while ...` or `label: loop ...`"
                    .to_string(),
            ])
    }
}

/// Error when a try expression is used (not yet supported)
// TODO: Remove this error when try expressions are fully implemented
pub struct TryExpressionNotSupportedError {
    pub span: Span,
}

impl IntoDiagnostic for TryExpressionNotSupportedError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("try expressions are not yet supported")
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("try expression used here"),
            ])
    }
}
