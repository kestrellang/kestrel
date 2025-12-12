use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when a function is missing a body (and it's required).
pub struct FunctionMissingBodyError {
    pub span: Span,
    pub function_name: String,
}

impl IntoDiagnostic for FunctionMissingBodyError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("function '{}' requires a body", self.function_name))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("function declared without body")
            ])
    }
}

