use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

#[derive(Debug, Clone)]
pub struct UninitializedVariableAccessError {
    pub span: Span,
    pub variable_name: String,
}

impl IntoDiagnostic for UninitializedVariableAccessError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("access to uninitialized variable '{}'", self.variable_name))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("uninitialized variable access here"),
            ])
    }
}
