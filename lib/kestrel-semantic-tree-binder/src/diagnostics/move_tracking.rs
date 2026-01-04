//! Move tracking errors.
//!
//! Errors related to use-after-move of non-copyable values.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when using a value that has been moved.
pub struct UseAfterMoveError {
    /// Span where the moved value is used
    pub use_span: Span,
    /// Name of the variable
    pub name: String,
    /// Span where the move occurred
    pub moved_at: Span,
}

impl IntoDiagnostic for UseAfterMoveError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("use of moved value `{}`", self.name))
            .with_labels(vec![
                Label::primary(self.use_span.file_id, self.use_span.range())
                    .with_message("value used here after move"),
                Label::secondary(self.moved_at.file_id, self.moved_at.range())
                    .with_message("value moved here"),
            ])
            .with_notes(vec![
                "non-copyable values can only be used once".to_string(),
                "help: consider cloning the value if it implements Clone".to_string(),
            ])
    }
}

/// Error when using a value that may have been moved (conditionally moved).
pub struct MaybeMovedError {
    /// Span where the potentially moved value is used
    pub use_span: Span,
    /// Name of the variable
    pub name: String,
    /// Span where the potential move occurred
    pub moved_at: Span,
}

impl IntoDiagnostic for MaybeMovedError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("value `{}` may have been moved", self.name))
            .with_labels(vec![
                Label::primary(self.use_span.file_id, self.use_span.range())
                    .with_message("value used here, but may have been moved"),
                Label::secondary(self.moved_at.file_id, self.moved_at.range())
                    .with_message("value potentially moved here"),
            ])
            .with_notes(vec![
                "value was moved in one branch but not another".to_string(),
                "help: ensure value is moved in all branches, or clone before the conditional"
                    .to_string(),
            ])
    }
}
