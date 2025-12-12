use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when a protocol method has a body.
pub struct ProtocolMethodHasBodyError {
    pub span: Span,
    pub method_name: String,
    pub protocol_name: String,
}

impl IntoDiagnostic for ProtocolMethodHasBodyError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "protocol method '{}' in '{}' cannot have a body",
                self.method_name, self.protocol_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("body not allowed in protocol method"),
            ])
    }
}
