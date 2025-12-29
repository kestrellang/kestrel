use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

pub struct GuardLetElseMustDivergeError {
    pub span: Span,
}

impl IntoDiagnostic for GuardLetElseMustDivergeError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("guard-let else block must diverge")
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("else block does not diverge"),
            ])
            .with_notes(vec![
                "the else block must end with return, break, or continue".to_string(),
            ])
    }
}
