//! Diagnostics for extern FFI safety validation

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when an extern function parameter or return type doesn't conform to FFISafe.
pub struct TypeNotFFISafeError {
    pub span: Span,
    pub ty: String,
    pub context: String, // "parameter" or "return type"
}

impl IntoDiagnostic for TypeNotFFISafeError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "{} type '{}' does not conform to FFISafe",
                self.context, self.ty
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("type is not FFI-safe"),
            ])
            .with_notes(vec![
                "only types conforming to FFISafe can cross FFI boundaries".to_string(),
                "consider using Pointer[T] or primitive types".to_string(),
            ])
    }
}
