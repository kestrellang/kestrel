use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Warning for unreachable code
pub struct UnreachableCodeWarning {
    /// Span of the unreachable code
    pub span: Span,
    /// What caused the code to be unreachable
    pub reason: UnreachableReason,
}

#[derive(Clone, Copy)]
pub enum UnreachableReason {
    AfterReturn,
    AfterBreak,
    AfterContinue,
    AfterInfiniteLoop,
}

impl UnreachableReason {
    pub fn description(&self) -> &'static str {
        match self {
            UnreachableReason::AfterReturn => "after return statement",
            UnreachableReason::AfterBreak => "after break statement",
            UnreachableReason::AfterContinue => "after continue statement",
            UnreachableReason::AfterInfiniteLoop => "after infinite loop",
        }
    }
}

impl IntoDiagnostic for UnreachableCodeWarning {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::warning()
            .with_message(format!("unreachable code {}", self.reason.description()))
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())
                .with_message("this code will never execute")])
    }
}
