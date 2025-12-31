//! Deinit-related diagnostic errors.
//!
//! Errors and warnings related to deinit declarations and RAII semantics.

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

/// Warning when a Copyable type has a deinit.
///
/// This is allowed but potentially confusing - the deinit will run for each copy.
pub struct CopyableWithDeinitWarning {
    /// Span of the deinit declaration
    pub deinit_span: Span,
    /// Name of the struct
    pub struct_name: String,
}

impl IntoDiagnostic for CopyableWithDeinitWarning {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::warning()
            .with_message(format!(
                "struct `{}` is Copyable but has deinit",
                self.struct_name
            ))
            .with_labels(vec![Label::primary(
                self.deinit_span.file_id,
                self.deinit_span.range(),
            )
            .with_message("deinit will run for each copy")])
            .with_notes(vec![
                "consider marking the struct as `not Copyable` if it manages resources".to_string(),
            ])
    }
}
