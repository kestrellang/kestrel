//! Diagnostics for irrefutable pattern warnings.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Warning when an irrefutable pattern is used in an if-let expression.
///
/// An irrefutable pattern always matches, making the if-let unnecessary.
/// The else branch will never execute.
///
/// Example:
/// ```ignore
/// if let x = value {    // Warning: `x` always matches
///     x
/// } else {
///     0                 // This never executes
/// }
/// ```
#[derive(Debug, Clone)]
pub struct IrrefutableIfLetWarning {
    /// Span of the pattern
    pub pattern_span: Span,
    /// Human-readable description of the pattern
    pub pattern_description: String,
}

impl IntoDiagnostic for IrrefutableIfLetWarning {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::warning()
            .with_message("irrefutable pattern in if-let")
            .with_labels(vec![
                Label::primary(self.pattern_span.file_id, self.pattern_span.range()).with_message(
                    format!("pattern `{}` will always match", self.pattern_description),
                ),
            ])
            .with_notes(vec![
                "consider using a regular `let` binding instead".to_string(),
            ])
    }
}

/// Warning when an irrefutable pattern appears before the last arm in a match.
///
/// An irrefutable pattern makes all subsequent arms unreachable.
///
/// Example:
/// ```ignore
/// match value {
///     x => ...,        // Warning: `x` always matches
///     0 => ...,        // Unreachable!
/// }
/// ```
#[derive(Debug, Clone)]
pub struct IrrefutableMatchArmWarning {
    /// Span of the irrefutable pattern
    pub pattern_span: Span,
    /// Human-readable description of the pattern
    pub pattern_description: String,
    /// Number of unreachable arms after this one
    pub unreachable_count: usize,
}

impl IntoDiagnostic for IrrefutableMatchArmWarning {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let arms_word = if self.unreachable_count == 1 {
            "arm"
        } else {
            "arms"
        };
        Diagnostic::warning()
            .with_message("irrefutable pattern in match makes subsequent arms unreachable")
            .with_labels(vec![
                Label::primary(self.pattern_span.file_id, self.pattern_span.range()).with_message(
                    format!("pattern `{}` will always match", self.pattern_description),
                ),
            ])
            .with_notes(vec![format!(
                "{} {} after this pattern will never be reached",
                self.unreachable_count, arms_word
            )])
    }
}
