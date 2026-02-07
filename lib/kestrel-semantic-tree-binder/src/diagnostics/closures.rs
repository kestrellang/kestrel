//! Closure-related errors.
//!
//! Errors related to closure capture and escape analysis.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when a closure with captures is returned from a function.
///
/// Closures that capture variables have their environment allocated on the stack.
/// When such closures escape (are returned), the environment pointer becomes
/// dangling, causing undefined behavior.
///
/// TODO: Remove this restriction once heap allocation for closure environments
/// is implemented.
pub struct CapturingClosureEscapeError {
    pub closure_span: Span,
    pub return_span: Span,
    pub captured_names: Vec<String>,
}

impl IntoDiagnostic for CapturingClosureEscapeError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let captured_list = self.captured_names.join(", ");
        Diagnostic::error()
            .with_message("cannot return a closure that captures variables")
            .with_labels(vec![
                Label::primary(self.closure_span.file_id, self.closure_span.range())
                    .with_message(format!("this closure captures: {}", captured_list)),
                Label::secondary(self.return_span.file_id, self.return_span.range())
                    .with_message("returned here"),
            ])
            .with_notes(vec![
                "closures that capture variables cannot escape their defining function".into(),
                "hint: use a non-capturing closure or pass captured values as parameters".into(),
            ])
    }
}
