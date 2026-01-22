//! Diagnostics for for-loop pattern errors.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when a refutable pattern is used in a for-loop.
#[derive(Debug, Clone)]
pub struct RefutableForLoopPatternError {
    pub pattern_span: Span,
    pub pattern_description: String,
}

impl IntoDiagnostic for RefutableForLoopPatternError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "refutable pattern `{}` in for loop",
                self.pattern_description
            ))
            .with_labels(vec![
                Label::primary(self.pattern_span.file_id, self.pattern_span.range())
                    .with_message("pattern must be irrefutable"),
            ])
            .with_notes(vec![
                "for-loop patterns must match all items from the iterator".to_string(),
                "consider using `while let` if you want to match a refutable pattern".to_string(),
            ])
    }
}
