//! Attribute-related warnings.
//!
//! Warnings emitted for unknown or invalid attributes.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Warning when an unknown attribute is used.
pub struct UnknownAttributeWarning {
    /// The name of the unknown attribute
    pub name: String,
    /// The span of the attribute
    pub span: Span,
}

impl IntoDiagnostic for UnknownAttributeWarning {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::warning()
            .with_message(format!("unknown attribute '{}'", self.name))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("unknown attribute"),
            ])
    }
}
