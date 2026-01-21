//! Diagnostics for duplicate callable detection.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when multiple callables have the same signature (name + labels).
pub struct DuplicateCallableError {
    /// Display representation of the duplicate key (e.g., "foo(x:, y:)")
    pub signature: String,
    /// Kind of callable (function, initializer, subscript)
    pub kind: &'static str,
    /// Span of the first definition
    pub first_span: Span,
    /// Span of the duplicate definition
    pub duplicate_span: Span,
}

impl IntoDiagnostic for DuplicateCallableError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "duplicate {} signature: {}",
                self.kind, self.signature
            ))
            .with_labels(vec![
                Label::primary(self.duplicate_span.file_id, self.duplicate_span.range())
                    .with_message("duplicate definition"),
                Label::secondary(self.first_span.file_id, self.first_span.range())
                    .with_message("first defined here"),
            ])
    }
}
