//! Diagnostics for builtin marker protocol validation

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when a builtin protocol is required to be a marker protocol but has required members.
pub struct BuiltinMustBeMarkerError {
    pub span: Span,
    pub feature_name: String,
}

impl IntoDiagnostic for BuiltinMustBeMarkerError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "@builtin(.{}) must be a marker protocol (no required methods or types)",
                self.feature_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("protocol has required members"),
            ])
    }
}
