//! Diagnostics for disallowed enum conformance validation

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when an enum tries to conform to a protocol that disallows enum conformance.
pub struct ProtocolDisallowsEnumConformanceError {
    pub span: Span,
    pub enum_name: String,
    pub protocol_name: String,
}

impl IntoDiagnostic for ProtocolDisallowsEnumConformanceError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "enum '{}' cannot conform to protocol '{}'",
                self.enum_name, self.protocol_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("enums cannot conform to this protocol"),
            ])
            .with_notes(vec![format!(
                "'{}' only allows struct conformance",
                self.protocol_name
            )])
    }
}
