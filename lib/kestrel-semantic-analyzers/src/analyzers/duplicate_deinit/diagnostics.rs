//! Diagnostics for duplicate deinit detection

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when a struct has multiple deinit declarations.
pub struct DuplicateDeinitError {
    /// Span of the first deinit declaration
    pub first_span: Span,
    /// Span of the duplicate deinit declaration
    pub duplicate_span: Span,
    /// Name of the struct
    pub struct_name: String,
}

impl IntoDiagnostic for DuplicateDeinitError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "struct `{}` already has a deinit",
                self.struct_name
            ))
            .with_labels(vec![
                Label::secondary(self.first_span.file_id, self.first_span.range())
                    .with_message("first deinit defined here"),
                Label::primary(self.duplicate_span.file_id, self.duplicate_span.range())
                    .with_message("duplicate deinit"),
            ])
    }
}
