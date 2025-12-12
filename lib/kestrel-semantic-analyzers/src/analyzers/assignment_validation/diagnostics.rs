use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

pub struct CannotAssignToImmutableError {
    pub span: Span,
    pub variable_name: String,
}

impl IntoDiagnostic for CannotAssignToImmutableError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "cannot assign to immutable variable '{}'",
                self.variable_name
            ))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message("assignment to immutable variable")])
            .with_notes(vec![
                "consider changing this to `var` if you need to mutate it".to_string(),
            ])
    }
}

pub struct CannotAssignToExpressionError {
    pub span: Span,
}

impl IntoDiagnostic for CannotAssignToExpressionError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("cannot assign to this expression")
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message("not a valid assignment target")])
            .with_notes(vec!["assignment target must be a variable or field".to_string()])
    }
}

pub struct CannotAssignToImmutableFieldError {
    pub span: Span,
    pub field_name: String,
}

impl IntoDiagnostic for CannotAssignToImmutableFieldError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("cannot assign to immutable field '{}'", self.field_name))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message("assignment to immutable field")])
            .with_notes(vec!["this field was declared with `let`, not `var`".to_string()])
    }
}

