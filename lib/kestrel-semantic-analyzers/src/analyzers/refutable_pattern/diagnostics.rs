//! Diagnostics for refutable pattern errors.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when a refutable pattern is used in a let/var binding.
///
/// Example:
/// ```ignore
/// let 42 = x           // Error: literal pattern is refutable
/// let .Some(v) = opt   // Error: enum variant is refutable
/// ```
#[derive(Debug, Clone)]
pub struct RefutablePatternError {
    /// Span of the pattern
    pub pattern_span: Span,
    /// Human-readable description of the pattern
    pub pattern_description: String,
}

impl IntoDiagnostic for RefutablePatternError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("refutable pattern in let binding")
            .with_labels(vec![
                Label::primary(self.pattern_span.file_id, self.pattern_span.range()).with_message(
                    format!("pattern `{}` might not match", self.pattern_description),
                ),
            ])
            .with_notes(vec![
                "consider using `if let` or `match` instead".to_string(),
            ])
    }
}
