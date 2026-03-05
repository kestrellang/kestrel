//! Diagnostics for parent protocol conformance validation

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when a struct conforms to a protocol that inherits from another protocol,
/// but doesn't also declare conformance to the parent protocol.
pub struct MissingParentProtocolConformanceError {
    pub span: Span,
    pub struct_name: String,
    pub child_protocol: String,
    pub parent_protocol: String,
}

impl IntoDiagnostic for MissingParentProtocolConformanceError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "'{}' conforms to '{}' but not its parent protocol '{}'",
                self.struct_name, self.child_protocol, self.parent_protocol
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message(format!("must also conform to '{}'", self.parent_protocol)),
            ])
            .with_notes(vec![
                format!(
                    "protocol '{}' inherits from '{}', so '{}' must explicitly conform to both",
                    self.child_protocol, self.parent_protocol, self.struct_name
                ),
                format!("add ': {}' to the conformance list", self.parent_protocol),
            ])
    }
}
