//! Diagnostics for exhaustiveness checking.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when a match expression is not exhaustive.
///
/// Example:
/// ```ignore
/// match color {
///     .Red => 1,
///     .Green => 2
///     // Error: missing .Blue
/// }
/// ```
#[derive(Debug, Clone)]
pub struct NonExhaustiveMatchError {
    /// Span of the match keyword
    pub match_span: Span,
    /// Human-readable description of missing patterns
    pub missing_patterns: Vec<String>,
}

impl IntoDiagnostic for NonExhaustiveMatchError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let missing = if self.missing_patterns.len() == 1 {
            format!("pattern `{}` not covered", self.missing_patterns[0])
        } else if self.missing_patterns.len() <= 3 {
            format!("patterns {} not covered", self.missing_patterns.join(", "))
        } else {
            format!(
                "patterns {}, and {} more not covered",
                self.missing_patterns[..3].join(", "),
                self.missing_patterns.len() - 3
            )
        };

        Diagnostic::error()
            .with_message("non-exhaustive match expression")
            .with_labels(vec![Label::primary(
                self.match_span.file_id,
                self.match_span.range(),
            )
            .with_message(missing)])
            .with_notes(vec![
                "ensure all possible values are covered".to_string(),
                "consider adding a wildcard pattern `_` as a catch-all".to_string(),
            ])
    }
}

/// Error when a match expression has no arms but the type is inhabited.
#[derive(Debug, Clone)]
pub struct EmptyMatchError {
    /// Span of the match expression
    pub match_span: Span,
    /// The type being matched
    pub scrutinee_type: String,
}

impl IntoDiagnostic for EmptyMatchError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("empty match expression")
            .with_labels(vec![Label::primary(
                self.match_span.file_id,
                self.match_span.range(),
            )
            .with_message(format!(
                "type `{}` is inhabited and requires at least one arm",
                self.scrutinee_type
            ))])
    }
}

/// Warning when a match arm is unreachable.
///
/// Example:
/// ```ignore
/// match color {
///     .Red => 1,
///     _ => 0,
///     .Green => 2  // Warning: unreachable
/// }
/// ```
#[derive(Debug, Clone)]
pub struct UnreachablePatternWarning {
    /// Span of the unreachable pattern
    pub pattern_span: Span,
}

impl IntoDiagnostic for UnreachablePatternWarning {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::warning()
            .with_message("unreachable pattern")
            .with_labels(vec![Label::primary(
                self.pattern_span.file_id,
                self.pattern_span.range(),
            )
            .with_message("this pattern will never be matched")])
            .with_notes(vec![
                "this arm is redundant because earlier patterns cover all its cases".to_string(),
            ])
    }
}
