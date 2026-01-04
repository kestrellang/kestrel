use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

pub struct MissingReturnError {
    pub span: Span,
    pub func_name: String,
}

impl IntoDiagnostic for MissingReturnError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "function '{}' does not return a value on all code paths",
                self.func_name
            ))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message("this function has a non-unit return type")])
            .with_notes(vec![
                "all code paths must end with a return statement or a value expression".to_string(),
            ])
    }
}
